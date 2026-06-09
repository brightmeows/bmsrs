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

    for variant in &data_enum.variants {
        let variant_ident = &variant.ident;
        let tokens: Vec<String> = variant
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("bms_token"))
            .filter_map(|a| {
                if let Ok(lit) = a.parse_args::<syn::LitStr>() {
                    Some(lit.value())
                } else {
                    None
                }
            })
            .collect();

        if tokens.is_empty() {
            return syn::Error::new_spanned(
                variant,
                "each variant must have at least one #[bms_token(\"...\")] attribute",
            )
            .to_compile_error();
        }

        // SAFETY: non-emptiness verified above by the `if tokens.is_empty()` check.
        #[expect(clippy::indexing_slicing, reason = "pre-validated non-empty")]
        let first_token = &tokens[0];
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

    let enum_path = quote! { #enum_name #ty_generics };

    quote! {
        impl #impl_generics ::std::str::FromStr for #enum_name #ty_generics
        #where_clause
        {
            type Err = crate::ParseBmsValueError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.trim() {
                    #from_str_arms
                    _ => Err(crate::ParseBmsValueError),
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
