//! Proc-macro helpers for `bms-tokenizer` — provides `#[derive(BmsTokenAttr)]`
//! for mapping header enum variants to their BMS command strings.

mod codegen;
mod parse;

use proc_macro::TokenStream;
use syn::DeriveInput;

use crate::codegen::generate_impl;
use crate::parse::parse_bms_token_attr;

/// Derive `try_match_header`, `format_header` and `__bms_dispatch` on a
/// header sub-enum.
///
/// Each variant must be annotated with `#[bms_token("...")]` specifying the
/// BMS command pattern (e.g., `#[bms_token("#BPM {value}")]`).
#[proc_macro_derive(BmsTokenAttr, attributes(bms_token))]
pub fn derive_bms_token_attr(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let enum_name = &input.ident;
    let generics = &input.generics;

    let syn::Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(enum_name, "BmsToken can only be derived on enums")
            .to_compile_error()
            .into();
    };

    // Collect templates for each variant.
    let mut templates: Vec<Vec<parse::BmsTokenTemplate>> = Vec::new();
    for variant in &data_enum.variants {
        let mut variant_templates = Vec::new();
        for attr in &variant.attrs {
            if attr.path().is_ident("bms_token") {
                match parse_bms_token_attr(attr) {
                    Ok(tmpl) => variant_templates.push(tmpl),
                    Err(err) => {
                        return err.to_compile_error().into();
                    }
                }
            }
        }
        templates.push(variant_templates);
    }

    generate_impl(enum_name, generics, data_enum, &templates).into()
}
