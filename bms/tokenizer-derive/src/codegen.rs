//! Code generator for `#[derive(BmsTokenAttr)]`.

#![expect(clippy::indexing_slicing, reason = "bounded by checked iteration")]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;

use crate::parse::BmsTokenTemplate;

/// Return a detail string literal for domain types whose parse failures
/// benefit from an expected-range annotation.
fn type_detail_ts(ty: &syn::Type) -> TokenStream {
    let detail: &'static str = match ty {
        syn::Type::Path(type_path) => {
            let seg = type_path.path.segments.last();
            match seg.map(|s| s.ident.to_string()).as_deref() {
                Some("LnType") => " (expected 1 or 2)",
                Some("LnMode") => " (expected 1, 2, or 3)",
                Some("DifficultyLevel") => " (expected 1-5)",
                _ => "",
            }
        }
        _ => "",
    };
    let lit = syn::LitStr::new(detail, ty.span());
    quote! { #lit }
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
                command: &str,
                command_raw: &'header str,
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

/// Generate the body of `try_match_header`.
fn generate_try_match_body(
    data_enum: &syn::DataEnum,
    templates: &[Vec<BmsTokenTemplate>],
    fallbacks: &[bool],
) -> TokenStream {
    let mut exact_arms = TokenStream::new();
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
                let arm = generate_exact_match_arm(variant, tmpl, is_fallback);
                exact_arms.extend(arm);
            }
        }
    }

    quote! {
        match command {
            #exact_arms
            _ => {}
        }

        #indexed_checks

        Ok(None)
    }
}

/// Generate a single match arm for a non-indexed variant.
fn generate_exact_match_arm(
    variant: &syn::Variant,
    tmpl: &BmsTokenTemplate,
    is_fallback: bool,
) -> TokenStream {
    let cmd_str = &tmpl.command;
    let context_str = format!("{}{}", tmpl.prefix, tmpl.command);
    let command_ident = &variant.ident;

    // Literal-value template: match both command AND value literally.
    if let Some(literal) = &tmpl.value_literal {
        return quote! {
            #cmd_str if value == #literal => { return Ok(Some(Self::#command_ident)); }
        };
    }

    if !tmpl.has_value() {
        return quote! {
            #cmd_str => { return Ok(Some(Self::#command_ident)); }
        };
    }

    match &variant.fields {
        syn::Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => {
            exact_unnamed_arm(
                command_ident,
                &fields_unnamed.unnamed[0].ty,
                cmd_str,
                &context_str,
                is_fallback,
            )
        }
        syn::Fields::Named(fields_named) => {
            let value_field_name = tmpl.value_field.as_deref().unwrap_or("value");
            let Some(field) = fields_named
                .named
                .iter()
                .find(|f| f.ident.as_ref().is_some_and(|n| n == value_field_name))
            else {
                return quote! {
                    #cmd_str => { return Ok(Some(Self::#command_ident)); }
                };
            };
            exact_named_field_arm(command_ident, field, cmd_str, &context_str, is_fallback)
        }
        _ => {
            quote! {
                #cmd_str => { return Ok(Some(Self::#command_ident)); }
            }
        }
    }
}

/// Generate a match arm for a tuple variant with exactly one field.
fn exact_unnamed_arm(
    command_ident: &syn::Ident,
    field_ty: &syn::Type,
    cmd_str: &str,
    context_str: &str,
    is_fallback: bool,
) -> TokenStream {
    if is_str_ref(field_ty) {
        quote! {
            #cmd_str => { return Ok(Some(Self::#command_ident(value))); }
        }
    } else if is_fallback {
        quote! {
            #cmd_str => {
                let Some(__value) = <#field_ty as crate::BmsValue>::parse(value) else {
                    return Ok(None);
                };
                return Ok(Some(Self::#command_ident(__value)));
            }
        }
    } else {
        let detail = type_detail_ts(field_ty);
        quote! {
            #cmd_str => {
                let __value: #field_ty = value.parse().map_err(|_|
                    crate::BmsTokenizeError::InvalidValue {
                        context: #context_str,
                        value,
                        detail: #detail,
                    }
                )?;
                return Ok(Some(Self::#command_ident(__value)));
            }
        }
    }
}

/// Generate a match arm for a struct variant, parsing the value field.
fn exact_named_field_arm(
    command_ident: &syn::Ident,
    field: &syn::Field,
    cmd_str: &str,
    context_str: &str,
    is_fallback: bool,
) -> TokenStream {
    let field_ty = &field.ty;
    let field_ident = &field.ident;
    if is_str_ref(field_ty) {
        quote! {
            #cmd_str => {
                return Ok(Some(Self::#command_ident {
                    #field_ident: value,
                }));
            }
        }
    } else if is_fallback {
        quote! {
            #cmd_str => {
                let Some(#field_ident) = <#field_ty as crate::BmsValue>::parse(value) else {
                    return Ok(None);
                };
                return Ok(Some(Self::#command_ident {
                    #field_ident,
                }));
            }
        }
    } else {
        let detail = type_detail_ts(field_ty);
        quote! {
            #cmd_str => {
                let #field_ident: #field_ty = value.parse().map_err(|_|
                    crate::BmsTokenizeError::InvalidValue {
                        context: #context_str,
                        value,
                        detail: #detail,
                    }
                )?;
                return Ok(Some(Self::#command_ident {
                    #field_ident,
                }));
            }
        }
    }
}

/// Generate prefix/length checks and variant construction for an indexed
/// command.  Uses `command_raw` for index slicing so the error can store the
/// correct portion of the input.
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
        if command.len() == #expected_len && command.starts_with(#cmd_prefix) {
            let __idx = &command_raw[#idx_start..];
            #variant_construction
        }
    }
}

/// Build the body that constructs the variant, parsing `__idx` and `value`
/// from `command_raw` (which has the correct lifetime for error storage).
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
            // Parse index — `__idx` is `&'header str` because it slices from
            // `command_raw`, which carries the input's lifetime.
            field_inits.extend(quote! {
                let #field_ident: #field_ty = __idx.parse().map_err(|_|
                    crate::BmsTokenizeError::InvalidValue {
                        context: #context_str,
                        value: __idx,
                        detail: "",
                    }
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
            let detail = type_detail_ts(field_ty);
            field_inits.extend(quote! {
                let #field_ident: #field_ty = value.parse().map_err(|_|
                    crate::BmsTokenizeError::InvalidValue {
                        context: #context_str,
                        value,
                        detail: #detail,
                    }
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
            // Variant has no #[bms_token] attrs — handled by the fallback
            // loop below.
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
    // These are hand-parsed variants (e.g., parameterized commands handled in
    // parse_header_line). Without them, the match self would be non-exhaustive.
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
    // Literal-value template: return the literal string directly.
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
///
/// Like [`variant_to_pattern`] but uses `{ .. }` / `(..)` instead of
/// naming each field, avoiding "unused variable" warnings in fallback arms.
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
        // Skip variants with #[bms_fallback] (e.g., the catch-all Fallback).
        if variant
            .attrs
            .iter()
            .any(|a| a.path().is_ident("bms_fallback"))
        {
            continue;
        }
        // Only tuple variants with one field (the inner sub-enum).
        let syn::Fields::Unnamed(fields) = &variant.fields else {
            continue;
        };
        if fields.unnamed.len() != 1 {
            continue;
        }

        let variant_ident = &variant.ident;
        let inner_type = &fields.unnamed[0].ty;

        dispatch_arms.extend(quote! {
            if let Some(v) = <#inner_type>::try_match_header(command, command_raw, value)? {
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
                command: &str,
                command_raw: &'header str,
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
