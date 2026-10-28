//! `#[derive(BmsTokenAttr)]` 的代码生成器。

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;

use crate::parse::{BmsTokenTemplate, Placeholder};

/// 从枚举定义中提取的泛型参数上下文，打包传递以避免在代码生成调用链中
/// 超出参数数量限制。
struct GenericsCtx<'a> {
    /// 枚举首个类型参数的 ident（如 `C`），如有。
    type_param_ident: Option<&'a syn::Ident>,
    /// 枚举的首个生命周期，如有。
    #[expect(dead_code, reason = "retained for API compatibility with downstream")]
    first_lifetime: Option<&'a syn::Lifetime>,
    /// 在生成签名中使用的合成 `'header` 生命周期。
    #[expect(dead_code, reason = "retained for API compatibility with downstream")]
    header_lifetime: &'a syn::Lifetime,
}

/// 当属性为 `#[doc(hidden)]` 时返回 `true`。
fn is_doc_hidden(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("doc")
        && attr
            .meta
            .require_list()
            .is_ok_and(|list| list.tokens.to_string().contains("hidden"))
}

/// 为头部子枚举生成 `BmsToken` 的 impl 块。
pub fn generate_impl(
    enum_name: &syn::Ident,
    generics: &syn::Generics,
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
    fallbacks: &[bool],
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let first_lifetime_param = generics.lifetimes().next();
    let first_lifetime = first_lifetime_param.map(|lt| &lt.lifetime);
    let type_param = generics.type_params().next();
    let type_param_ident = type_param.map(|tp| &tp.ident);

    // 构建 try_match_header 的 where 子句：
    // 仅需确保 'header 不短于枚举的生命周期参数。
    let header_where = first_lifetime_param.map_or_else(TokenStream::new, |lt| {
        let lifetime = &lt.lifetime;
        quote! { where 'header: #lifetime }
    });

    let header_lifetime = syn::Lifetime::new("'header", proc_macro2::Span::call_site());
    let generics_ctx = GenericsCtx {
        type_param_ident,
        first_lifetime,
        header_lifetime: &header_lifetime,
    };
    let error_type = quote! { crate::BmsTokenizeError };
    let try_match_body = generate_try_match_body(data_enum, templates, fallbacks, &generics_ctx);
    let format_body = generate_format_body(data_enum, templates);

    let format_where = TokenStream::new();

    quote! {
        impl #impl_generics #enum_name #ty_generics #where_clause {
            /// Try to parse a `(command, value)` pair into a variant of this
            /// enum.
            #[doc(hidden)]
            #[must_use]
            pub fn try_match_header<'header>(
                command: &'header str,
                value: &'header str,
            ) -> Result<Option<Self>, #error_type>
            #header_where
            {
                #try_match_body
            }

            /// Format this variant back to a `(command_token, value)` pair.
            #[doc(hidden)]
            #[must_use]
            pub fn format_header(&self) -> (String, String)
            #format_where
            {
                match self {
                    #format_body
                }
            }
        }
    }
}

/// `if/else if` 链中用于精确（非索引）匹配的单个分支。
struct ExactBranch {
    /// 条件表达式（如 `command.eq_ignore_ascii_case("TITLE")`）。
    condition: TokenStream,
    /// 条件匹配时要执行的体。
    body: TokenStream,
}

/// 生成 `try_match_header` 的函数体。
fn generate_try_match_body(
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
    fallbacks: &[bool],
    generics: &GenericsCtx,
) -> TokenStream {
    let mut exact_branches: Vec<ExactBranch> = Vec::new();
    let mut indexed_checks = TokenStream::new();

    for (variant_idx, variant) in data_enum.variants.iter().enumerate() {
        // 跳过 #[doc(hidden)] 变体（如来自 PhantomData 的 _Phantom）。
        if variant.attrs.iter().any(is_doc_hidden) {
            continue;
        }
        let Some(variant_templates) = templates.get(variant_idx) else {
            continue;
        };
        let is_fallback = fallbacks.get(variant_idx).copied().unwrap_or(false);

        for tmpl in variant_templates {
            // 校验占位符一致性：全部 Named 或全部 Unnamed。
            if let Some(err) = check_placeholder_consistency(tmpl, variant) {
                return err;
            }

            if tmpl.is_indexed() {
                let check = generate_indexed_match(variant, tmpl, is_fallback, generics);
                indexed_checks.extend(check);
            } else {
                let branches = generate_exact_branches(variant, tmpl, is_fallback, generics);
                exact_branches.extend(branches);
            }
        }
    }

    let exact_chain = build_if_else_chain(&exact_branches);

    quote! {
        #exact_chain

        #indexed_checks

        Ok(None)
    }
}

/// 根据分支构建 `if cond { body } else if cond { body } ...` 链。
fn build_if_else_chain(branches: &[ExactBranch]) -> TokenStream {
    if branches.is_empty() {
        return TokenStream::new();
    }

    let mut result = TokenStream::new();
    for (i, branch) in branches.iter().enumerate() {
        let cond = &branch.condition;
        let body = &branch.body;
        if i == 0 {
            result.extend(quote! { if #cond { #body } });
        } else {
            result.extend(quote! { else if #cond { #body } });
        }
    }
    result
}

/// 为非索引变体生成精确匹配分支。
fn generate_exact_branches(
    variant: &syn::Variant,
    tmpl: &BmsTokenTemplate,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> Vec<ExactBranch> {
    let cmd_str = &tmpl.command;
    let context_str = format!("{}{}", tmpl.prefix, tmpl.command);
    let command_ident = &variant.ident;

    // 字面值模板：命令与值均按字面匹配。
    if let Some(literal) = &tmpl.value_literal {
        return vec![ExactBranch {
            condition: quote! { command.eq_ignore_ascii_case(#cmd_str) && value == #literal },
            body: quote! { return Ok(Some(Self::#command_ident)); },
        }];
    }

    if !tmpl.has_value() {
        return vec![ExactBranch {
            condition: quote! { command.eq_ignore_ascii_case(#cmd_str) },
            body: quote! { return Ok(Some(Self::#command_ident)); },
        }];
    }

    let (condition, body) = match &variant.fields {
        syn::Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => {
            // 长度已确认为 1，first() 必然为 Some。
            #[expect(clippy::unwrap_used, reason = "len() == 1 confirmed above")]
            let field = fields_unnamed.unnamed.first().unwrap();
            exact_unnamed_branch(
                command_ident,
                &field.ty,
                cmd_str,
                &context_str,
                is_fallback,
                generics,
            )
        }
        syn::Fields::Named(fields_named) => {
            let value_field_name = tmpl.value_field_name();
            let Some(field) = fields_named
                .named
                .iter()
                .find(|f| f.ident.as_ref().is_some_and(|n| n == value_field_name))
            else {
                return vec![ExactBranch {
                    condition: quote! { command.eq_ignore_ascii_case(#cmd_str) },
                    body: quote! { return Ok(Some(Self::#command_ident)); },
                }];
            };
            exact_named_field_branch(
                command_ident,
                field,
                cmd_str,
                &context_str,
                is_fallback,
                generics,
            )
        }
        syn::Fields::Unnamed(_) | syn::Fields::Unit => {
            return vec![ExactBranch {
                condition: quote! { command.eq_ignore_ascii_case(#cmd_str) },
                body: quote! { return Ok(Some(Self::#command_ident)); },
            }];
        }
    };

    vec![ExactBranch { condition, body }]
}

/// 为恰好含一个字段的 tuple 变体生成分支。
fn exact_unnamed_branch(
    command_ident: &syn::Ident,
    field_ty: &syn::Type,
    cmd_str: &str,
    context_str: &str,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> (TokenStream, TokenStream) {
    let cond = quote! { command.eq_ignore_ascii_case(#cmd_str) };
    let __value = format_ident!("__value");
    let init = gen_field_parse(
        &quote!(#__value),
        field_ty,
        &quote!(value),
        context_str,
        is_fallback,
        generics,
    );
    let body = quote! {
        #init
        return Ok(Some(Self::#command_ident(#__value)));
    };
    (cond, body)
}

/// 为 struct 变体生成分支，解析 value 字段。
fn exact_named_field_branch(
    command_ident: &syn::Ident,
    field: &syn::Field,
    cmd_str: &str,
    context_str: &str,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> (TokenStream, TokenStream) {
    let field_ty = &field.ty;
    let field_ident = &field.ident;
    let cond = quote! { command.eq_ignore_ascii_case(#cmd_str) };
    let init = gen_field_parse(
        &quote!(#field_ident),
        field_ty,
        &quote!(value),
        context_str,
        is_fallback,
        generics,
    );
    let body = quote! {
        #init
        return Ok(Some(Self::#command_ident {
            #field_ident,
        }));
    };
    (cond, body)
}

/// 为索引命令生成前缀/长度检查与变体构造。
fn generate_indexed_match(
    variant: &syn::Variant,
    tmpl: &BmsTokenTemplate,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> TokenStream {
    let context_str = format!("{}{}", tmpl.prefix, tmpl.command);
    let base_len = tmpl.command.len();
    let cmd_prefix = &tmpl.command;
    let idx_start = base_len;
    let expected_len = base_len + 2;

    let variant_construction = if tmpl.is_unnamed_id() && tmpl.is_unnamed_value() {
        // tuple 变体，id 与 value 均为匿名。
        build_indexed_tuple_body(variant, &context_str, is_fallback, generics)
    } else {
        let id_field_name = tmpl.id_field_name();
        build_indexed_variant_body(variant, id_field_name, &context_str, is_fallback, generics)
    };

    quote! {
        if command.len() == #expected_len && command[..#base_len].eq_ignore_ascii_case(#cmd_prefix) {
            let __idx = &command[#idx_start..];
            #variant_construction
        }
    }
}

/// 构造变体的体，解析 `__idx` 与 `value`。
fn build_indexed_variant_body(
    variant: &syn::Variant,
    id_field_name: &str,
    context_str: &str,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> TokenStream {
    let command_ident = &variant.ident;

    let syn::Fields::Named(fields_named) = &variant.fields else {
        return quote! { return Ok(Some(Self::#command_ident)); };
    };

    let mut field_inits = TokenStream::new();

    for field in &fields_named.named {
        let field_ident = &field.ident;
        let field_name = field_ident
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        let field_ty = &field.ty;

        // id 字段始终使用 FromStr + IntoTokensError（绝不回退）。
        let is_id = field_name == id_field_name;
        let value_src = if is_id { quote!(__idx) } else { quote!(value) };
        let fb = if is_id { false } else { is_fallback };

        field_inits.extend(gen_field_parse(
            &quote!(#field_ident),
            field_ty,
            &value_src,
            context_str,
            fb,
            generics,
        ));
    }

    let field_names: Vec<_> = fields_named.named.iter().map(|f| &f.ident).collect();

    quote! {
        #field_inits
        return Ok(Some(Self::#command_ident {
            #(#field_names),*
        }));
    }
}

/// 为 `format_header` 生成 match 分支。
fn generate_format_body(
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
) -> TokenStream {
    let mut arms = TokenStream::new();
    let mut has_doc_hidden = false;

    for (variant_idx, variant) in data_enum.variants.iter().enumerate() {
        if variant.attrs.iter().any(is_doc_hidden) {
            has_doc_hidden = true;
            continue;
        }
        let Some(variant_templates) = templates.get(variant_idx) else {
            continue;
        };
        let Some(tmpl) = variant_templates.first() else {
            continue;
        };

        let cmd_token = format!("{}{}", tmpl.prefix, tmpl.command);
        let pattern = variant_to_pattern(variant);

        let cmd_expr = if tmpl.is_indexed() {
            let base_lit = syn::LitStr::new(&cmd_token, variant.span());
            if tmpl.is_unnamed_id() {
                // 匿名 id → 使用 __bind_0（首个 tuple 字段）。
                quote! { format!("{}{}", #base_lit, __bind_0) }
            } else {
                let id_field_str = tmpl.id_field_name();
                let id_ident = format_ident!("{id_field_str}");
                quote! { format!("{}{}", #base_lit, #id_ident) }
            }
        } else {
            let cmd_lit = syn::LitStr::new(&cmd_token, variant.span());
            quote! { #cmd_lit.to_owned() }
        };

        let val_expr = generate_format_value_expr(variant, tmpl);

        arms.extend(quote! {
            #pattern => (#cmd_expr, #val_expr),
        });
    }

    // 不带任何 #[bms_token] 属性的变体的回退分支。
    for (variant_idx, variant) in data_enum.variants.iter().enumerate() {
        if variant.attrs.iter().any(is_doc_hidden) {
            continue;
        }
        let Some(variant_templates) = templates.get(variant_idx) else {
            continue;
        };
        if !variant_templates.is_empty() {
            continue;
        }
        let pattern = variant_to_wildcard_pattern(variant);
        let name_str = variant.ident.to_string();
        let name_lit = syn::LitStr::new(&name_str, variant.span());
        arms.extend(quote! {
            #pattern => (#name_lit.to_owned(), String::new()),
        });
    }

    // 针对 #[doc(hidden)] 变体（如来自 PhantomData 的 _Phantom）的兜底分支。
    if has_doc_hidden {
        arms.extend(quote! { _ => unreachable!(), });
    }

    arms
}

/// 生成 `format_header` match 分支的值侧表达式。
fn generate_format_value_expr(variant: &syn::Variant, tmpl: &BmsTokenTemplate) -> TokenStream {
    if let Some(literal) = &tmpl.value_literal {
        let lit = syn::LitStr::new(literal, variant.span());
        return quote! { #lit.to_owned() };
    }

    let value_field_name = match &tmpl.value_field {
        Some(Placeholder::Named(name)) => name.clone(),
        Some(Placeholder::Unnamed) => {
            // 匿名 value：匿名字段由 Fields::Unnamed 分支处理。
            "value".to_owned()
        }
        None => return quote! { String::new() },
    };

    match &variant.fields {
        syn::Fields::Unnamed(fields_unnamed) => {
            // 使用最后一个匿名字段作为 value（索引命令中索引 0 = id，
            // 索引 1 或 0 = value）。
            let last = fields_unnamed.unnamed.len().saturating_sub(1);
            let bind = format_ident!("__bind_{last}");
            quote! { #bind.to_string() }
        }
        syn::Fields::Named(fields_named) => {
            let field_ident = format_ident!("{value_field_name}");
            if fields_named
                .named
                .iter()
                .any(|f| f.ident.as_ref().is_some_and(|n| n == &value_field_name))
            {
                quote! { #field_ident.to_string() }
            } else {
                quote! { String::new() }
            }
        }
        syn::Fields::Unit => quote! { String::new() },
    }
}

/// 将变体定义转换为 `match self` 模式。
fn variant_to_pattern(variant: &syn::Variant) -> TokenStream {
    let ident = &variant.ident;
    match &variant.fields {
        syn::Fields::Named(fields_named) => {
            let field_names: Vec<_> = fields_named.named.iter().map(|f| &f.ident).collect();
            quote! { Self::#ident { #(#field_names),* } }
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            let bindings: Vec<_> = (0..fields_unnamed.unnamed.len())
                .map(|i| format_ident!("__bind_{i}"))
                .collect();
            quote! { Self::#ident(#(#bindings),*) }
        }
        syn::Fields::Unit => quote! { Self::#ident },
    }
}

/// 将变体定义转换为 `match self` 通配符模式。
fn variant_to_wildcard_pattern(variant: &syn::Variant) -> TokenStream {
    let ident = &variant.ident;
    match &variant.fields {
        syn::Fields::Named(_) => quote! { Self::#ident { .. } },
        syn::Fields::Unnamed(_) => quote! { Self::#ident(..) },
        syn::Fields::Unit => quote! { Self::#ident },
    }
}

/// 当字段类型为给定类型参数 ident 时返回 `true`。
///
/// 检查类型路径的末段是否等于 `type_param_ident`。
/// 用于区分字符串字段与普通字段（在移除 C 泛型前曾用于匹配 `C` 参数）。
fn is_type_param(ty: &syn::Type, type_param_ident: &syn::Ident) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    type_path
        .path
        .get_ident()
        .is_some_and(|id| id == type_param_ident)
}

/// 当字段类型为 `String` 时返回 `true`。
fn is_string_type(ty: &syn::Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    type_path.path.is_ident("String")
}

/// 当字段类型为 `&str`（任意生命周期）时返回 `true`。
fn is_str_ref(ty: &syn::Type) -> bool {
    let syn::Type::Reference(type_ref) = ty else {
        return false;
    };
    matches!(&*type_ref.elem, syn::Type::Path(p) if p.path.is_ident("str"))
}

/// 生成从一个字符串源初始化字段的 `let` 绑定。
///
/// 五种情形：
/// - 字段类型匹配 `type_param_ident`：`let #ident = From::from(#value_src);`
///   （使用 `From::from` 而非 `BmsStr::from_borrowed`，因为 `BmsStr` 的约束
///   会通过其 blanket 实现引发 E0283；`From::from` 是父 trait 约束的等价形式）
/// - 字段类型为 `String`：`let #ident = From::from(#value_src);`
/// - `&str` 字段：直接赋值
/// - 回退字段：`let Some(#ident) = <T as BmsValue>::parse(#value_src) else { return Ok(None); };`
/// - 其他字段：`let #ident: T = #value_src.parse()...?;`（通过 `IntoTokensError`）
fn gen_field_parse(
    ident: &TokenStream,
    field_ty: &syn::Type,
    value_src: &TokenStream,
    context_str: &str,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> TokenStream {
    let GenericsCtx {
        type_param_ident,
        first_lifetime: _,
        header_lifetime: _,
    } = generics;
    // 情形 1：字段类型匹配泛型类型参数 → From::from
    // （仅当 enum 仍有泛型参数时匹配，当前统一使用 String 故通常不触发）。
    if let Some(tp_ident) = type_param_ident
        && is_type_param(field_ty, tp_ident)
    {
        return quote! {
            let #ident = ::std::convert::From::from(#value_src);
        };
    }

    // 情形 2：String 字段 → From::from(value)（owned 转换）。
    if is_string_type(field_ty) {
        return quote! {
            let #ident = ::std::convert::From::from(#value_src);
        };
    }

    // 情形 3：&str 字段 → 直接赋值。
    if is_str_ref(field_ty) {
        return quote! { let #ident = #value_src; };
    }

    // 情形 4/5：回退（BmsValue）或 FromStr。
    if is_fallback {
        let bms_value_path = quote! { crate::BmsValue };
        quote! {
            let Some(#ident) = <#field_ty as #bms_value_path>::parse(#value_src) else {
                return Ok(None);
            };
        }
    } else {
        let into_tokens_path = quote! { crate::IntoTokensError };
        let value_arg = quote! { #value_src.to_owned() };
        quote! {
            let #ident: #field_ty = #value_src.parse().map_err(|e|
                <<#field_ty as ::std::str::FromStr>::Err as #into_tokens_path>::into_error(
                    e, #context_str, #value_arg,
                )
            )?;
        }
    }
}

/// 校验模板的占位符是否一致：全部 Named 或全部 Unnamed。
/// 混合时返回含 `compile_error!` 的 `Some(TokenStream)`，否则返回 `None`。
fn check_placeholder_consistency(
    tmpl: &BmsTokenTemplate,
    variant: &syn::Variant,
) -> Option<TokenStream> {
    let id_is_named = tmpl.id_field.as_ref().map(Placeholder::is_named);
    let value_is_named = tmpl.value_field.as_ref().map(Placeholder::is_named);

    // 若两者都存在且不一致，则为混合。
    let mixed = matches!(
        (id_is_named, value_is_named),
        (Some(id_named), Some(val_named)) if id_named != val_named
    );

    mixed.then(|| {
        let msg = "cannot mix named and unnamed placeholders in the same #[bms_token] template";
        syn::Error::new_spanned(variant, msg).to_compile_error()
    })
}

/// 为带匿名占位符的索引命令（`#BPM{} {}`）构造 tuple 变体的体。
/// 首个字段为索引值，第二个（或仅剩的）字段为 value。
fn build_indexed_tuple_body(
    variant: &syn::Variant,
    context_str: &str,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> TokenStream {
    let command_ident = &variant.ident;

    let syn::Fields::Unnamed(fields_unnamed) = &variant.fields else {
        return quote! { return Ok(Some(Self::#command_ident)); };
    };

    let mut field_inits = TokenStream::new();
    let mut field_exprs = Vec::new();

    for (i, field) in fields_unnamed.unnamed.iter().enumerate() {
        let field_ty = &field.ty;
        let bind = format_ident!("__f{i}");

        // 首个字段为索引（来自命令后缀） —— 永不回退。
        let is_id = i == 0;
        let value_src = if is_id { quote!(__idx) } else { quote!(value) };
        let fb = if is_id { false } else { is_fallback };

        field_inits.extend(gen_field_parse(
            &quote!(#bind),
            field_ty,
            &value_src,
            context_str,
            fb,
            generics,
        ));
        field_exprs.push(quote!(#bind));
    }

    quote! {
        #field_inits
        return Ok(Some(Self::#command_ident(#(#field_exprs),*)));
    }
}

/// 在顶层 `BmsHeader` 枚举上生成 `try_match_header`。
///
/// 生成的方法按声明顺序依次调度到各子枚举的 `try_match_header`。
/// 带 `#[bms_fallback]` 的变体（如兜底的 `Fallback` 变体）会被跳过。
pub fn generate_header_dispatch(
    enum_name: &syn::Ident,
    generics: &syn::Generics,
    data_enum: &syn::DataEnum,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let first_lifetime = generics.lifetimes().next();

    let header_where = first_lifetime.map_or_else(TokenStream::new, |lt| {
        let lifetime = &lt.lifetime;
        quote! { where 'header: #lifetime }
    });

    let error_type = quote! { crate::BmsTokenizeError };
    let mut dispatch_arms = TokenStream::new();

    for variant in &data_enum.variants {
        // 跳过 #[doc(hidden)] 变体（如 PhantomData）与 #[bms_fallback]。
        if variant
            .attrs
            .iter()
            .any(|a| a.path().is_ident("bms_fallback") || is_doc_hidden(a))
        {
            continue;
        }
        let syn::Fields::Unnamed(fields) = &variant.fields else {
            continue;
        };
        if fields.unnamed.len() != 1 {
            continue;
        }

        let variant_ident = &variant.ident;
        // 长度已确认为 1，first() 必然为 Some。
        #[expect(clippy::unwrap_used, reason = "len() == 1 confirmed above")]
        let inner_type = &fields.unnamed.first().unwrap().ty;

        dispatch_arms.extend(quote! {
            if let Some(v) = <#inner_type>::try_match_header(command, value)? {
                return Ok(Some(Self::#variant_ident(v)));
            }
        });
    }

    quote! {
        impl #impl_generics #enum_name #ty_generics #where_clause {
            /// Dispatch to each sub-enum's `try_match_header` in declaration
            /// order.  Returns `Ok(None)` when no sub-enum recognises the
            /// command.
            #[doc(hidden)]
            #[must_use]
            pub fn try_match_header<'header>(
                command: &'header str,
                value: &'header str,
            ) -> Result<Option<Self>, #error_type>
            #header_where
            {
                #dispatch_arms
                Ok(None)
            }
        }
    }
}
