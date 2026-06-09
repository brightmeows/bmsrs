//! Code generator for `#[derive(BmsTokenAttr)]`.

#![expect(clippy::indexing_slicing, reason = "bounded by checked iteration")]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;

use crate::parse::BmsTokenTemplate;

/// Generate the `BmsToken` impl block for a header sub-enum.
pub fn generate_impl(
    enum_name: &syn::Ident,
    generics: &syn::Generics,
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
    fallbacks: &[bool],
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let first_lifetime = generics.lifetimes().next();
    let header_bound = first_lifetime.map(|lt| {
        let lifetime = &lt.lifetime;
        quote! { where 'header: #lifetime }
    });
    let header_where = header_bound.unwrap_or_default();

    let try_match_body = generate_try_match_body(data_enum, templates, fallbacks);
    let format_body = generate_format_body(data_enum, templates);

    quote! {
        impl #impl_generics #enum_name #ty_generics #where_clause {
            /// Try to parse a `(command, value)` pair into a variant of this
            /// enum.
            #[doc(hidden)]
            #[must_use]
            pub fn try_match_header<'header>(
                command: &'header str,
                value: &'header str,
            ) -> Result<Option<Self>, crate::BmsTokenizeError<'header>>
            #header_where
            {
                #try_match_body
            }

            /// Format this variant back to a `(command_token, value)` pair.
            #[doc(hidden)]
            #[must_use]
            pub fn format_header(&self) -> (String, String) {
                match self {
                    #format_body
                }
            }
        }
    }
}

/// A single branch in the `if/else if` chain for exact (non-indexed) matching.
struct ExactBranch {
    /// The condition expression (e.g., `command.eq_ignore_ascii_case("TITLE")`).
    condition: TokenStream,
    /// The body to execute when the condition matches.
    body: TokenStream,
}

/// Generate the body of `try_match_header`.
fn generate_try_match_body(
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
    fallbacks: &[bool],
) -> TokenStream {
    let mut exact_branches: Vec<ExactBranch> = Vec::new();
    let mut indexed_checks = TokenStream::new();

    for (variant_idx, variant) in data_enum.variants.iter().enumerate() {
        let Some(variant_templates) = templates.get(variant_idx) else {
            continue;
        };
        let is_fallback = fallbacks.get(variant_idx).copied().unwrap_or(false);

        for tmpl in variant_templates {
            if tmpl.is_indexed() {
                let check = generate_indexed_match(variant, tmpl, is_fallback);
                indexed_checks.extend(check);
            } else {
                let branches = generate_exact_branches(variant, tmpl, is_fallback);
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

/// Build an `if cond { body } else if cond { body } ...` chain from branches.
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
) -> Vec<ExactBranch> {
    let cmd_str = &tmpl.command;
    let context_str = format!("{}{}", tmpl.prefix, tmpl.command);
    let command_ident = &variant.ident;

    // Literal-value template: match both command AND value literally.
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
            exact_unnamed_branch(
                command_ident,
                &fields_unnamed.unnamed[0].ty,
                cmd_str,
                &context_str,
                is_fallback,
                variant,
            )
        }
        syn::Fields::Named(fields_named) => {
            let value_field_name = tmpl.value_field.as_deref().unwrap_or("value");
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
                variant,
            )
        }
        _ => {
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
    _variant: &syn::Variant,
) -> (TokenStream, TokenStream) {
    let cond = quote! { command.eq_ignore_ascii_case(#cmd_str) };
    let map_body = |body| (cond, body);

    if is_str_ref(field_ty) {
        return map_body(quote! { return Ok(Some(Self::#command_ident(value))); });
    }

    if is_fallback {
        return map_body(quote! {
            let Some(__value) = <#field_ty as crate::BmsValue>::parse(value) else {
                return Ok(None);
            };
            return Ok(Some(Self::#command_ident(__value)));
        });
    }

    // All other fields: unified IntoTokensError conversion.
    map_body(quote! {
        let __value: #field_ty = value.parse().map_err(|e|
            <<#field_ty as ::std::str::FromStr>::Err as crate::IntoTokensError>::into_error(
                e, #context_str, value,
            )
        )?;
        return Ok(Some(Self::#command_ident(__value)));
    })
}

/// Generate a branch for a struct variant, parsing the value field.
fn exact_named_field_branch(
    command_ident: &syn::Ident,
    field: &syn::Field,
    cmd_str: &str,
    context_str: &str,
    is_fallback: bool,
    _variant: &syn::Variant,
) -> (TokenStream, TokenStream) {
    let field_ty = &field.ty;
    let field_ident = &field.ident;
    let cond = quote! { command.eq_ignore_ascii_case(#cmd_str) };
    let map_body = |body| (cond, body);

    if is_str_ref(field_ty) {
        return map_body(quote! {
            return Ok(Some(Self::#command_ident {
                #field_ident: value,
            }));
        });
    }

    if is_fallback {
        return map_body(quote! {
            let Some(#field_ident) = <#field_ty as crate::BmsValue>::parse(value) else {
                return Ok(None);
            };
            return Ok(Some(Self::#command_ident {
                #field_ident,
            }));
        });
    }

    // All other fields: unified IntoTokensError conversion.
    map_body(quote! {
        let #field_ident: #field_ty = value.parse().map_err(|e|
            <<#field_ty as ::std::str::FromStr>::Err as crate::IntoTokensError>::into_error(
                e, #context_str, value,
            )
        )?;
        return Ok(Some(Self::#command_ident {
            #field_ident,
        }));
    })
}

/// Generate prefix/length checks and variant construction for an indexed
/// command.
fn generate_indexed_match(
    variant: &syn::Variant,
    tmpl: &BmsTokenTemplate,
    is_fallback: bool,
) -> TokenStream {
    let context_str = format!("{}{}", tmpl.prefix, tmpl.command);
    let base_len = tmpl.command.len();
    let cmd_prefix = &tmpl.command;
    let idx_start = base_len;
    let expected_len = base_len + 2;

    let id_field_name = tmpl.id_field.as_deref().unwrap_or("id");

    let variant_construction =
        build_indexed_variant_body(variant, id_field_name, &context_str, is_fallback);

    quote! {
        if command.len() == #expected_len && command[..#base_len].eq_ignore_ascii_case(#cmd_prefix) {
            let __idx = &command[#idx_start..];
            #variant_construction
        }
    }
}

/// Build the body that constructs the variant, parsing `__idx` and `value`.
fn build_indexed_variant_body(
    variant: &syn::Variant,
    id_field_name: &str,
    context_str: &str,
    is_fallback: bool,
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

        if field_name == id_field_name {
            // Index (command suffix) parsing: unified IntoTokensError.
            field_inits.extend(quote! {
                let #field_ident: #field_ty = __idx.parse().map_err(|e|
                    <<#field_ty as ::std::str::FromStr>::Err
                        as crate::IntoTokensError>::into_error(
                        e, #context_str, __idx,
                    )
                )?;
            });
        } else if is_str_ref(field_ty) {
            field_inits.extend(quote! {
                let #field_ident = value;
            });
        } else if is_fallback {
            field_inits.extend(quote! {
                let Some(#field_ident) = <#field_ty as crate::BmsValue>::parse(value) else {
                    return Ok(None);
                };
            });
        } else {
            // Value fields: unified IntoTokensError.
            field_inits.extend(quote! {
                let #field_ident: #field_ty = value.parse().map_err(|e|
                    <<#field_ty as ::std::str::FromStr>::Err
                        as crate::IntoTokensError>::into_error(
                        e, #context_str, value,
                    )
                )?;
            });
        }
    }

    let field_names: Vec<_> = fields_named.named.iter().map(|f| &f.ident).collect();

    quote! {
        #field_inits
        return Ok(Some(Self::#command_ident {
            #(#field_names),*
        }));
    }
}

/// Generate the match arms for `format_header`.
fn generate_format_body(
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
) -> TokenStream {
    let mut arms = TokenStream::new();

    for (variant_idx, variant) in data_enum.variants.iter().enumerate() {
        let Some(variant_templates) = templates.get(variant_idx) else {
            continue;
        };
        let Some(tmpl) = variant_templates.first() else {
            continue;
        };

        let cmd_token = format!("{}{}", tmpl.prefix, tmpl.command);
        let pattern = variant_to_pattern(variant);

        let cmd_expr = if tmpl.is_indexed() {
            let id_field_str = tmpl.id_field.as_deref().unwrap_or("id");
            let id_ident = format_ident!("{id_field_str}");
            let base_lit = syn::LitStr::new(&cmd_token, variant.span());
            quote! { format!("{}{}", #base_lit, #id_ident) }
        } else {
            let cmd_lit = syn::LitStr::new(&cmd_token, variant.span());
            quote! { #cmd_lit.to_owned() }
        };

        let val_expr = generate_format_value_expr(variant, tmpl);

        arms.extend(quote! {
            #pattern => (#cmd_expr, #val_expr),
        });
    }

    // Fallback arms for variants WITHOUT any #[bms_token] attrs.
    for (variant_idx, variant) in data_enum.variants.iter().enumerate() {
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

    arms
}

/// Generate the value-side expression of a `format_header` match arm.
fn generate_format_value_expr(variant: &syn::Variant, tmpl: &BmsTokenTemplate) -> TokenStream {
    if let Some(literal) = &tmpl.value_literal {
        let lit = syn::LitStr::new(literal, variant.span());
        return quote! { #lit.to_owned() };
    }

    let value_field_name = match &tmpl.value_field {
        Some(name) => name.clone(),
        None => return quote! { String::new() },
    };

    match &variant.fields {
        syn::Fields::Unnamed(_) => quote! { __bind_0.to_string() },
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

/// `true` if the type is a (possibly lifetime-qualified) `&str`.
fn is_str_ref(ty: &syn::Type) -> bool {
    let syn::Type::Reference(ty_ref) = ty else {
        return false;
    };
    let syn::Type::Path(type_path) = &*ty_ref.elem else {
        return false;
    };
    type_path.path.is_ident("str")
}

/// Generate `try_match_header` on the top-level `BmsHeader` enum.
///
/// The generated method dispatches to each sub-enum's `try_match_header`
/// in declaration order.  Variants annotated with `#[bms_fallback]`
/// (e.g., the catch-all `Fallback` variant) are skipped.
pub fn generate_header_dispatch(
    enum_name: &syn::Ident,
    generics: &syn::Generics,
    data_enum: &syn::DataEnum,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let first_lifetime = generics.lifetimes().next();
    let header_bound = first_lifetime.map(|lt| {
        let lifetime = &lt.lifetime;
        quote! { where 'header: #lifetime }
    });
    let header_where = header_bound.unwrap_or_default();

    let mut dispatch_arms = TokenStream::new();

    for variant in &data_enum.variants {
        if variant
            .attrs
            .iter()
            .any(|a| a.path().is_ident("bms_fallback"))
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
        let inner_type = &fields.unnamed[0].ty;

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
            ) -> Result<Option<Self>, crate::BmsTokenizeError<'header>>
            #header_where
            {
                #dispatch_arms
                Ok(None)
            }
        }
    }
}
