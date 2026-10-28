//! `#[derive(BmsIndex)]` proc-macro ——为 BMS 索引 newtype 生成标准 trait 实现。
//!
//! 输入：单字段元组结构体如 `WavIndex(pub BmsIndex)`。
//! 输出：`Clone`、`Copy`、`Debug`、`PartialEq`、`Eq`、`PartialOrd`、`Ord`、
//! `Hash`、`Deref`、`Display`、`From<Inner>`、`FromStr`、`TryFrom<&str>` 的实现。

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// 为 BMS 索引 newtype 生成标准 trait 实现的内部函数。
///
/// 由 `lib.rs` 中的 `#[proc_macro_derive(BmsIndexNewtype)]` 门面转发至此。
#[expect(
    clippy::too_many_lines,
    reason = "single function generates 13 trait impls; splitting would not improve clarity"
)]
pub fn derive_bms_index_impl(ts: TokenStream) -> TokenStream {
    let input = parse_macro_input!(ts as DeriveInput);
    let name = &input.ident;

    // 提取内部类型（元组结构体的第一个字段）
    let inner = match &input.data {
        syn::Data::Struct(ds) => match &ds.fields {
            syn::Fields::Unnamed(f)
                if f.unnamed.len() == 1
                    && let Some(first) = f.unnamed.first() =>
            {
                &first.ty
            }
            _ => {
                return syn::Error::new_spanned(
                    name,
                    "BmsIndex requires a tuple struct with exactly one field",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "BmsIndex can only be derived on structs")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        #[automatically_derived]
        impl ::core::clone::Clone for #name {
            #[inline]
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        #[automatically_derived]
        impl ::core::marker::Copy for #name {}

        #[automatically_derived]
        impl ::core::fmt::Debug for #name {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_tuple(::core::stringify!(#name))
                    .field(&self.0)
                    .finish()
            }
        }

        #[automatically_derived]
        impl ::core::cmp::PartialEq for #name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        #[automatically_derived]
        impl ::core::cmp::Eq for #name {}

        #[automatically_derived]
        impl ::core::cmp::PartialOrd for #name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        #[automatically_derived]
        impl ::core::cmp::Ord for #name {
            #[inline]
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        #[automatically_derived]
        impl ::core::hash::Hash for #name {
            #[inline]
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }

        #[automatically_derived]
        impl ::core::ops::Deref for #name {
            type Target = #inner;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[automatically_derived]
        impl ::core::fmt::Display for #name {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::write!(f, "{}", self.0)
            }
        }

        #[automatically_derived]
        impl ::core::convert::From<#inner> for #name {
            #[inline]
            fn from(inner: #inner) -> Self {
                Self(inner)
            }
        }

        #[automatically_derived]
        impl ::core::str::FromStr for #name {
            type Err = <#inner as ::core::str::FromStr>::Err;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                <#inner as ::core::str::FromStr>::from_str(s).map(Self)
            }
        }

        #[automatically_derived]
        impl ::core::convert::TryFrom<&str> for #name {
            type Error = <Self as ::core::str::FromStr>::Err;

            #[inline]
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };

    expanded.into()
}
