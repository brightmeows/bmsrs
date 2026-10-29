//! Proc-macro helper: `#[derive(BmsTokenAttr)]` handles every
//! `#[bms_token("...")]` pattern in a single derive.
//!
//! Three modes are detected automatically from the enum shape:
//!
//! | Mode | Trigger | Generated items |
//! |---|---|---|
//! | **Command** | A variant has `#[bms_token("#...")]` | `try_match_header` + `format_header` |
//! | **Literal** | A variant has `#[bms_token("0")]` | `FromStr` + `Display` |
//! | **Dispatch** | No `#[bms_token]`, all variants are single-field tuple variants | `BmsHeader::try_match_header` |
//!
//! In dispatch mode, variants marked `#[bms_fallback]` (the catch-all
//! `Fallback` variant) are excluded from forwarding. In command mode,
//! `#[bms_fallback]` makes a parse failure return `Ok(None)` (falling through
//! to `BmsHeaderFallback`) instead of `Err`.
//!
//! Generated code references `crate::BmsValue` and `crate::BmsTokenizeError`,
//! so consumers must provide these types at the crate root (bmsrs-bms-values
//! re-exports them from its `lib.rs`).

mod codegen;
mod index;
mod parse;
mod value_codegen;

use proc_macro::TokenStream;
use syn::DeriveInput;

use crate::codegen::{generate_header_dispatch, generate_impl};
use crate::parse::parse_bms_token_attr;
use crate::value_codegen::generate_bms_value_enum;

/// Unified derive for every `#[bms_token("...")]` pattern.
///
/// See the [crate-level documentation](self) for the three modes.
#[cfg_attr(
    test,
    expect(
        clippy::missing_inline_in_public_items,
        reason = "proc-macro entry point cannot be inlined"
    )
)]
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

    // Dispatch mode: no variant has #[bms_token], and all variants are
    // single-field tuple variants (such as Metadata(BmsHeaderMetadata)).
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

    // Literal or command mode: inspect the first #[bms_token] value.
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

/// Generate the standard trait implementations for a BMS index newtype.
///
/// The input must be a single-field tuple struct (such as
/// `WavIndex(pub BmsIndex)`).
///
/// Generated trait implementations: `Clone`, `Copy`, `Debug`, `PartialEq`,
/// `Eq`, `PartialOrd`, `Ord`, `Hash`, `Deref`, `Display`, `From<Inner>`,
/// `FromStr`, and `TryFrom<&str>`.
#[proc_macro_derive(BmsIndexNewtype)]
pub fn derive_bms_index(input: TokenStream) -> TokenStream {
    index::derive_bms_index_impl(input)
}
