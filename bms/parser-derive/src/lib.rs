//! Proc-macro 辅助工具 —— `#[derive(BmsTokenAttr)]` 在单个 derive 中
//! 处理所有 `#[bms_token("...")]` 模式。
//!
//! 根据枚举结构自动检测三种模式：
//!
//! | 模式 | 触发条件 | 生成的项 |
//! |---|---|---|
//! | **Command** | 变体带 `#[bms_token("#...")]` | `try_match_header` + `format_header` |
//! | **Literal** | 变体带 `#[bms_token("0")]` | `FromStr` + `Display` |
//! | **Dispatch** | 无 `#[bms_token]`，全部为单字段 tuple 变体 | `BmsHeader::try_match_header` |
//!
//! 在 dispatch 模式下，带 `#[bms_fallback]` 的变体会从转发中排除
//! （即兜底的 `Fallback` 变体）。在 command 模式下，`#[bms_fallback]`
//! 会使解析失败时返回 `Ok(None)`（穿透到 `BmsHeaderFallback`）而非 `Err`。
//!
//! 生成的代码会引用 `crate::BmsValue` 与 `crate::BmsTokenizeError`，
//! 因此消费方必须在 crate 根下提供这些类型（bms-tokenizer 会从其
//! `lib.rs` 中重新导出这些类型）。

mod codegen;
mod index;
mod parse;
mod value_codegen;

use proc_macro::TokenStream;
use syn::DeriveInput;

use crate::codegen::{generate_header_dispatch, generate_impl};
use crate::parse::parse_bms_token_attr;
use crate::value_codegen::generate_bms_value_enum;

/// 针对所有 `#[bms_token("...")]` 模式的统一 derive。
///
/// 三种工作模式的描述请参见 [crate 级文档](self)。
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

    // Dispatch 模式：所有变体都没有 #[bms_token]，且全部为单字段 tuple 变体
    // （如 Metadata(BmsHeaderMetadata)）。
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

    // Literal 还是 command 模式：查看首个 #[bms_token] 的值。
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

/// 为 BMS 索引 newtype 生成标准 trait 实现。
///
/// 输入必须是单字段元组结构体（如 `WavIndex(pub BmsIndex)`）。
///
/// 生成的 trait 实现：`Clone`、`Copy`、`Debug`、`PartialEq`、`Eq`、
/// `PartialOrd`、`Ord`、`Hash`、`Deref`、`Display`、`From<Inner>`、
/// `FromStr`、`TryFrom<&str>`。
#[proc_macro_derive(BmsIndexNewtype)]
pub fn derive_bms_index(input: TokenStream) -> TokenStream {
    index::derive_bms_index_impl(input)
}
