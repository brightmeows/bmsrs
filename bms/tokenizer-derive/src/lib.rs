//! Proc-macro helper — `#[derive(BmsTokenAttr)]` handles all
//! `#[bms_token("...")]` patterns in a single derive.
//!
//! Three modes are detected automatically from the enum structure:
//!
//! | Mode | Trigger | Generated items |
//! |---|---|---|
//! | **Command** | variants have `#[bms_token("#...")]` | `try_match_header` + `format_header` |
//! | **Literal** | variants have `#[bms_token("0")]` | `FromStr` + `Display` |
//! | **Dispatch** | no `#[bms_token]`, all single-tuple | `BmsHeader::try_match_header` |
//!
//! In dispatch mode, variants with `#[bms_fallback]` are excluded from
//! forwarding (the catch-all `Fallback` variant).  In command mode,
//! `#[bms_fallback]` causes parse failures to return `Ok(None)` (falling
//! through to `BmsHeaderFallback`) instead of `Err`.
//!
//! The generated code references `crate::BmsValue` and `crate::BmsTokenizeError`,
//! so the consumer must have these types in their crate root (bms-tokenizer
//! re-exports them from its `lib.rs`).

mod codegen;
mod parse;
mod value_codegen;

use proc_macro::TokenStream;
use syn::DeriveInput;

use crate::codegen::{generate_header_dispatch, generate_impl};
use crate::parse::parse_bms_token_attr;
use crate::value_codegen::generate_bms_value_enum;

/// Unified derive for all `#[bms_token("...")]` patterns.
///
/// See the [crate-level documentation](self) for a description of the three
/// operating modes.
#[proc_macro_derive(BmsTokenAttr, attributes(bms_token, bms_fallback))]
pub fn derive_bms_token_attr(input: TokenStream) -> TokenStream {
    #[expect(
        clippy::shadow_reuse,
        reason = "standard proc-macro shadow: TokenStream -> DeriveInput"
    )]
    let input = syn::parse_macro_input!(input as DeriveInput);

    let enum_name = &input.ident;
    let generics = &input.generics;

    let syn::Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(enum_name, "BmsTokenAttr can only be derived on enums")
            .to_compile_error()
            .into();
    };

    // Dispatch mode: no #[bms_token] on any variant, and all variants are
    // single-field tuple variants (like Metadata(BmsHeaderMetadata<'a>)).
    let has_bms_token = data_enum
        .variants
        .iter()
        .any(|v| v.attrs.iter().any(|a| a.path().is_ident("bms_token")));

    if !has_bms_token {
        let all_single_tuple = data_enum
            .variants
            .iter()
            .all(|v| matches!(&v.fields, syn::Fields::Unnamed(f) if f.unnamed.len() == 1));
        if all_single_tuple {
            return generate_header_dispatch(enum_name, generics, data_enum).into();
        }
        return syn::Error::new_spanned(
            enum_name,
            "BmsTokenAttr: each variant must have #[bms_token(...)] or be a \
             single-field tuple enum (dispatch mode)",
        )
        .to_compile_error()
        .into();
    }

    // Literal vs command mode: peek the first #[bms_token] value.
    let first_token = data_enum.variants.iter().find_map(|v| {
        v.attrs.iter().find_map(|a| {
            if a.path().is_ident("bms_token") {
                a.parse_args::<syn::LitStr>().ok()
            } else {
                None
            }
        })
    });

    match first_token {
        Some(tok) if tok.value().starts_with('#') || tok.value().starts_with('%') => {
            let mut templates: Vec<Vec<parse::BmsTokenTemplate>> = Vec::new();
            let mut fallbacks: Vec<bool> = Vec::new();
            for variant in &data_enum.variants {
                let mut variant_templates = Vec::new();
                for attr in &variant.attrs {
                    if attr.path().is_ident("bms_token") {
                        match parse_bms_token_attr(attr) {
                            Ok(tmpl) => variant_templates.push(tmpl),
                            Err(err) => return err.to_compile_error().into(),
                        }
                    }
                }
                templates.push(variant_templates);
                fallbacks.push(
                    variant
                        .attrs
                        .iter()
                        .any(|a| a.path().is_ident("bms_fallback")),
                );
            }
            generate_impl(enum_name, generics, data_enum, &templates, &fallbacks).into()
        }
        _ => generate_bms_value_enum(enum_name, generics, data_enum).into(),
    }
}
