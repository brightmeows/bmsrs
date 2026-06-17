//! Code generator for `#[derive(BmsValue)]` on domain enums.
//!
//! Each variant is annotated with one or more `#[bms_token("literal")]`
//! attributes specifying the string representations of that variant.
//! The derive generates `FromStr` (matching any of the tokens) and
//! `Display` (outputting the first token).

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

/// Generate `FromStr` and `Display` for a domain enum.
///
/// # Panics
///
/// Panics at compile time (via `compile_error!`) if a variant has no
/// `#[bms_token("...")]` attribute.
pub fn generate_bms_value_enum(
    enum_name: &syn::Ident,
    generics: &syn::Generics,
    data_enum: &syn::DataEnum,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut from_str_arms = TokenStream::new();
    let mut display_arms = TokenStream::new();

    let mut canonical_tokens: Vec<String> = Vec::new();

    for variant in &data_enum.variants {
        let variant_ident = &variant.ident;
        let tokens: Vec<String> = variant
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("bms_token"))
            .filter_map(|a| a.parse_args::<syn::LitStr>().ok().map(|lit| lit.value()))
            .collect();

        if tokens.is_empty() {
            return syn::Error::new_spanned(
                variant,
                "each variant must have at least one #[bms_token(\"...\")] attribute",
            )
            .to_compile_error();
        }

        // Non-emptiness verified above by the `if tokens.is_empty()` check.
        #[expect(clippy::indexing_slicing, reason = "pre-validated non-empty")]
        let first_token = &tokens[0];
        canonical_tokens.push(first_token.clone());
        display_arms.extend(quote! {
            Self::#variant_ident => write!(f, #first_token),
        });

        for token in &tokens {
            let lit = syn::LitStr::new(token, variant.span());
            from_str_arms.extend(quote! {
                #lit => Ok(Self::#variant_ident),
            });
        }
    }

    // Build a hint like "expected 1, 2, 3, or 4" from the canonical tokens.
    let hint = build_literal_enum_hint(&canonical_tokens);
    let hint_lit = syn::LitStr::new(&hint, data_enum.enum_token.span);

    let enum_path = quote! { #enum_name #ty_generics };

    quote! {
        impl #impl_generics ::std::str::FromStr for #enum_name #ty_generics
        #where_clause
        {
            type Err = crate::ParseBmsValueError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.trim() {
                    #from_str_arms
                    _ => Err(crate::ParseBmsValueError(#hint_lit)),
                }
            }
        }

        impl #impl_generics ::std::fmt::Display for #enum_path
        #where_clause
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #display_arms
                }
            }
        }
    }
}

/// Build a human-readable hint string listing the valid values.
///
/// Examples:
/// - `["1", "2"]` → `"expected 1 or 2"`
/// - `["1", "2", "3"]` → `"expected 1, 2, or 3"`
/// - `["1", "01"]` → `"expected 1, 01"`
fn build_literal_enum_hint(values: &[String]) -> String {
    let n = values.len();
    if n == 0 {
        return String::new();
    }

    let mut s = String::from("expected ");
    for (i, val) in values.iter().enumerate() {
        if i == n - 1 {
            // Last item: prepend "or" with correct separator.
            if n > 2 {
                s.push_str(", or ");
            } else if n == 2 {
                s.push_str(" or ");
            }
        } else if i > 0 {
            s.push_str(", ");
        }
        s.push_str(val);
    }
    s
}
