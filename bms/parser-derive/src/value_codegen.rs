//! 领域枚举上 `#[derive(BmsValue)]` 的代码生成器。
//!
//! 每个变体通过一个或多个 `#[bms_token("literal")]` 属性标注，指定该
//! 变体的字符串表示。derive 会生成 `FromStr`（匹配任一 token）与
//! `Display`（输出首个 token）。

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;

/// 为领域枚举生成 `FromStr` 与 `Display`。
///
/// # Panics
///
/// 若变体没有 `#[bms_token("...")]` 属性，则在编译期触发
/// `compile_error!`。
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

        let Some(first_token) = tokens.first() else {
            return syn::Error::new_spanned(
                variant,
                "each variant must have at least one #[bms_token(\"...\")] attribute",
            )
            .to_compile_error();
        };
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

    // 从规范 token 构建提示字符串，例如 "expected 1, 2, 3, or 4"。
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

/// 构建列出合法值的人类可读提示字符串。
///
/// 示例：
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
            // 最后一项：以正确的分隔符前置 "or"。
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
