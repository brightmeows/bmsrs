//! Code generator for `#[derive(BmsTokenAttr)]`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;

use crate::parse::{BmsTokenTemplate, Placeholder};

/// Generic parameter context extracted from the enum definition, bundled
/// together so the code-generation call chain stays under the argument-count
/// limit.
struct GenericsCtx<'a> {
    /// Ident of the enum's first type parameter (such as `C`), if any.
    type_param_ident: Option<&'a syn::Ident>,
    /// The enum's first lifetime, if any.
    #[expect(dead_code, reason = "retained for API compatibility with downstream")]
    first_lifetime: Option<&'a syn::Lifetime>,
    /// Synthetic `'header` lifetime used in generated signatures.
    #[expect(dead_code, reason = "retained for API compatibility with downstream")]
    header_lifetime: &'a syn::Lifetime,
}

/// Return `true` when the attribute is `#[doc(hidden)]`.
fn is_doc_hidden(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("doc")
        && attr
            .meta
            .require_list()
            .is_ok_and(|list| list.tokens.to_string().contains("hidden"))
}

/// Generate the `BmsToken` impl block for a header sub-enum.
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

    // Build the where clause for try_match_header:
    // it only needs to ensure that 'header outlives the enum's lifetime
    // parameters.
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

/// A single branch in the `if/else if` chain for exact (non-indexed) matching.
struct ExactBranch {
    /// Condition expression (for example,
    /// `command.eq_ignore_ascii_case("TITLE")`).
    condition: TokenStream,
    /// Body to execute when the condition matches.
    body: TokenStream,
}

/// Generate the body of `try_match_header`.
fn generate_try_match_body(
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
    fallbacks: &[bool],
    generics: &GenericsCtx,
) -> TokenStream {
    let mut exact_branches: Vec<ExactBranch> = Vec::new();
    let mut indexed_checks = TokenStream::new();

    for (variant_idx, variant) in data_enum.variants.iter().enumerate() {
        // Skip #[doc(hidden)] variants (such as _Phantom from PhantomData).
        if variant.attrs.iter().any(is_doc_hidden) {
            continue;
        }
        let Some(variant_templates) = templates.get(variant_idx) else {
            continue;
        };
        let is_fallback = fallbacks.get(variant_idx).copied().unwrap_or(false);

        for tmpl in variant_templates {
            // Validate placeholder consistency: all Named or all Unnamed.
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

/// Build an `if cond { body } else if cond { body } ...` chain from the
/// branches.
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

/// Generate exact-match branches for a non-indexed variant.
fn generate_exact_branches(
    variant: &syn::Variant,
    tmpl: &BmsTokenTemplate,
    is_fallback: bool,
    generics: &GenericsCtx,
) -> Vec<ExactBranch> {
    let cmd_str = &tmpl.command;
    let context_str = format!("{}{}", tmpl.prefix, tmpl.command);
    let command_ident = &variant.ident;

    // Literal-value template: match both command and value literally.
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
            // Length is confirmed to be 1, so first() is always Some.
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

/// Generate a branch for a tuple variant with exactly one field.
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

/// Generate a branch for a struct variant, parsing the value field.
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

/// Generate prefix/length checks and variant construction for an indexed
/// command.
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
        // Tuple variant with anonymous id and value.
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

/// Construct the variant body, parsing `__idx` and `value`.
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

        // The id field always uses FromStr + IntoTokensError (never falls
        // back).
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

/// Generate match arms for `format_header`.
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
                // Anonymous id: use __bind_0 (the first tuple field).
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

    // Fallback arms for variants without any #[bms_token] attribute.
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

    // Catch-all arm for #[doc(hidden)] variants (such as _Phantom from
    // PhantomData).
    if has_doc_hidden {
        arms.extend(quote! { _ => unreachable!(), });
    }

    arms
}

/// Generate the value-side expression for a `format_header` match arm.
fn generate_format_value_expr(variant: &syn::Variant, tmpl: &BmsTokenTemplate) -> TokenStream {
    if let Some(literal) = &tmpl.value_literal {
        let lit = syn::LitStr::new(literal, variant.span());
        return quote! { #lit.to_owned() };
    }

    let value_field_name = match &tmpl.value_field {
        Some(Placeholder::Named(name)) => name.clone(),
        Some(Placeholder::Unnamed) => {
            // Anonymous value: anonymous fields are handled by the
            // Fields::Unnamed branch.
            "value".to_owned()
        }
        None => return quote! { String::new() },
    };

    match &variant.fields {
        syn::Fields::Unnamed(fields_unnamed) => {
            // Use the last anonymous field as the value (for indexed commands,
            // index 0 is the id and index 1 or 0 is the value).
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

/// Convert a variant definition into a `match self` pattern.
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

/// Convert a variant definition into a `match self` wildcard pattern.
fn variant_to_wildcard_pattern(variant: &syn::Variant) -> TokenStream {
    let ident = &variant.ident;
    match &variant.fields {
        syn::Fields::Named(_) => quote! { Self::#ident { .. } },
        syn::Fields::Unnamed(_) => quote! { Self::#ident(..) },
        syn::Fields::Unit => quote! { Self::#ident },
    }
}

/// Return `true` when the field type is the given type-parameter ident.
///
/// Checks whether the last segment of the type path equals `type_param_ident`.
/// Used to distinguish string fields from ordinary fields (it matched the `C`
/// parameter before the C generic was removed).
fn is_type_param(ty: &syn::Type, type_param_ident: &syn::Ident) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    type_path
        .path
        .get_ident()
        .is_some_and(|id| id == type_param_ident)
}

/// Return `true` when the field type is `String`.
fn is_string_type(ty: &syn::Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    type_path.path.is_ident("String")
}

/// Return `true` when the field type is `&str` (any lifetime).
fn is_str_ref(ty: &syn::Type) -> bool {
    let syn::Type::Reference(type_ref) = ty else {
        return false;
    };
    matches!(&*type_ref.elem, syn::Type::Path(p) if p.path.is_ident("str"))
}

/// Generate a `let` binding that initializes a field from a string source.
///
/// Five cases:
/// - Field type matches `type_param_ident`: `let #ident = From::from(#value_src);`
///   (`From::from` is used instead of `BmsStr::from_borrowed` because
///   `BmsStr`'s blanket impl triggers E0283; `From::from` is the equivalent
///   parent-trait form)
/// - Field type is `String`: `let #ident = From::from(#value_src);`
/// - `&str` field: direct assignment
/// - Fallback field: `let Some(#ident) = <T as BmsValue>::parse(#value_src) else { return Ok(None); };`
/// - Other fields: `let #ident: T = #value_src.parse()...?;` (through
///   `IntoTokensError`)
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
    // Case 1: field type matches a generic type parameter, use From::from.
    // (Only matches when the enum still has generic parameters; the current
    // code uses String uniformly, so this usually does not trigger.)
    if let Some(tp_ident) = type_param_ident
        && is_type_param(field_ty, tp_ident)
    {
        return quote! {
            let #ident = ::std::convert::From::from(#value_src);
        };
    }

    // Case 2: String field, use From::from(value) (owned conversion).
    if is_string_type(field_ty) {
        return quote! {
            let #ident = ::std::convert::From::from(#value_src);
        };
    }

    // Case 3: &str field, direct assignment.
    if is_str_ref(field_ty) {
        return quote! { let #ident = #value_src; };
    }

    // Cases 4/5: fallback (BmsValue) or FromStr.
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

/// Validate that a template's placeholders are consistent: all Named or all
/// Unnamed. Returns `Some(TokenStream)` containing `compile_error!` when mixed,
/// otherwise `None`.
fn check_placeholder_consistency(
    tmpl: &BmsTokenTemplate,
    variant: &syn::Variant,
) -> Option<TokenStream> {
    let id_is_named = tmpl.id_field.as_ref().map(Placeholder::is_named);
    let value_is_named = tmpl.value_field.as_ref().map(Placeholder::is_named);

    // Mixed when both exist and disagree.
    let mixed = matches!(
        (id_is_named, value_is_named),
        (Some(id_named), Some(val_named)) if id_named != val_named
    );

    mixed.then(|| {
        let msg = "cannot mix named and unnamed placeholders in the same #[bms_token] template";
        syn::Error::new_spanned(variant, msg).to_compile_error()
    })
}

/// Construct the body of a tuple variant for an indexed command with anonymous
/// placeholders (`#BPM{} {}`). The first field is the index; the second (or
/// only remaining) field is the value.
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

        // The first field is the index (from the command suffix); it never
        // falls back.
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

/// Generate `try_match_header` on the top-level `BmsHeader` enum.
///
/// The generated method dispatches to each sub-enum's `try_match_header` in
/// declaration order. Variants marked `#[bms_fallback]` (such as the `Fallback`
/// variant) are skipped.
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
        // Skip #[doc(hidden)] variants (such as PhantomData) and
        // #[bms_fallback].
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
        // Length is confirmed to be 1, so first() is always Some.
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
