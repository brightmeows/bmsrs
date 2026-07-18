//! 由 chumsky 驱动的 BMSON 反序列化。
//!
//! 使用基于 chumsky 的 JSON 解析器进行诊断与错误恢复，随后通过
//! `serde_json::from_str` 反序列化为 [`bmson_def`] 类型
//! （当输入 JSON 格式良好时为零拷贝）。
//!
//! # 快速上手
//!
//! ```rust
//! # use bmson_de_chumsky::BmsonParser;
//! let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]}}"#;
//! if let Ok(bmson) = BmsonParser::parse(json) {
//!     assert_eq!(bmson.song_info.title, "T");
//! } else {
//!     unreachable!("valid bmson JSON should parse");
//! }
//! ```
//!
//! # 错误恢复
//!
//! 原始的 chumsky JSON 解析器（支持尾随逗号、缺失逗号、括号不匹配的
//! 错误恢复）可通过 [`json::parse_json`] 使用。若 [`BmsonParser::parse`] 因轻微的
//! JSON 问题而失败，可手动恢复该值：
//!
//! ```rust
//! # use bmson_de_chumsky::BmsonParser;
//! # use bmson_de_chumsky::json;
//! let malformed = r#"{"key": "value",}"#; // 尾随逗号
//! let (value, _errors) = json::parse_json(malformed);
//! if let Some(val) = value {
//!     # let cleaned = serde_json::to_string(&val).unwrap();
//!     let _result = BmsonParser::parse(&cleaned);
//! }
//! ```
//!
//! 解析入口通过零大小 [`BmsonParser`] 类型的关联方法暴露。
//!
//! [`parse`]: BmsonParser::parse

pub mod error;
pub mod json;

pub use error::BmsonDeError;

use bmson_def::DetectedVersion;

use crate::json::ParseError;

/// 将 `serde_json::Error` 包装为版本标记的 `BmsonDeError::Deserialize`。
fn deser_err(version: &'static str) -> impl FnOnce(serde_json::Error) -> BmsonDeError {
    move |e| BmsonDeError::Deserialize {
        version,
        message: e.to_string(),
    }
}

/// 将解析器错误列表格式化为以 `sep` 分隔的字符串。
fn join_errors(errors: &[ParseError<'_>], sep: &str) -> String {
    errors
        .iter()
        .map(|e| format!("{e}"))
        .collect::<Vec<_>>()
        .join(sep)
}

/// 零大小入口类型，通过关联方法 [`BmsonParser::parse`] 暴露 BMSON 解析。
///
/// # 用法
///
/// ```ignore
/// use bmson_de_chumsky::BmsonParser;
/// let bmson = BmsonParser::parse(json)?;
/// ```
///
/// 也可通过 `bmson_de_chumsky::BmsonParser::parse(json)` 直接引用。
pub struct BmsonParser;

impl BmsonParser {
    /// 将 BMSON JSON 字符串解析为 [`bmson_def::Bmson`]。
    ///
    /// 这是主入口。首先运行 chumsky 解析器进行校验，随后使用
    /// `serde_json::from_str` 完成实际的零拷贝反序列化，得到
    /// [`bmson_def::Bmson`]。
    ///
    /// 返回值从 `json` 借用（零拷贝）。若输入不符合 ECMA-404 规范的合法 JSON
    /// （例如存在尾随逗号），将返回 [`BmsonDeError::JsonParse`] 或
    /// [`BmsonDeError::Deserialize`]。
    ///
    /// 若需要对 JSON 解析进行细粒度控制（错误恢复、自定义诊断），可直接使用
    /// [`json::parse_json`]，再将清理后的结果传回本函数。
    ///
    /// # Errors
    ///
    /// 当 chumsky 解析器无法产生任何输出（致命解析错误）时，返回
    /// [`BmsonDeError::JsonParse`]。
    ///
    /// 当 `"version"` 字段存在但无法识别时，返回
    /// [`BmsonDeError::UnknownVersion`]。
    ///
    /// 当 `serde_json` 拒绝该 JSON 或缺少必填字段时，返回
    /// [`BmsonDeError::Deserialize`]。
    ///
    /// 当从旧版 v0 格式转换为统一 v2 格式失败（例如 `init_bpm` 非法）时，
    /// 返回 [`BmsonDeError::V0Conversion`]。
    pub fn parse(json: &str) -> Result<bmson_def::Bmson<'_>, BmsonDeError> {
        // 1. 运行 chumsky 解析器进行校验。
        let (value, errors) = json::parse_json(json);
        let had_output = value.is_some();

        // 2. 若解析器完全未产生输出，无论各错误如何归类都返回解析错误。
        if !had_output {
            let (_warnings, _recovered, fatal) = json::classify_errors(errors, false);
            let msg = join_errors(&fatal, "\n");
            return Err(BmsonDeError::JsonParse(msg));
        }

        // 3. 从原始文本中检测 bmson 版本。
        let version = bmson_def::DetectedVersion::detect(json)?;

        // 4. 反序列化；失败时附带 chumsky 诊断信息。
        deserialize_by_version(json, version).map_err(|err| {
            if errors.is_empty() {
                return err;
            }
            match err {
                BmsonDeError::Deserialize {
                    version: deser_version,
                    message,
                } => {
                    let diag = join_errors(&errors, "; ");
                    BmsonDeError::Deserialize {
                        version: deser_version,
                        message: format!("{message}\n  (parser diagnostics: {diag})"),
                    }
                }
                other => other,
            }
        })
    }
}

/// 内部：分派到版本特定的反序列化。
fn deserialize_by_version<'a>(
    json: &'a str,
    version: DetectedVersion,
) -> Result<bmson_def::Bmson<'a>, BmsonDeError> {
    match version {
        DetectedVersion::V2 => {
            serde_json::from_str::<bmson_def::Bmson<'a>>(json).map_err(deser_err("v2.0.0"))
        }
        DetectedVersion::V1 => {
            let v1: bmson_def::v1::Bmson<'a> =
                serde_json::from_str(json).map_err(deser_err("v1.0.0"))?;
            Ok(bmson_def::Bmson::from(v1))
        }
        DetectedVersion::V0 => {
            let v0: bmson_def::v0::Bmson<'a> =
                serde_json::from_str(json).map_err(deser_err("v0.2.1"))?;
            Ok(bmson_def::Bmson::try_from(v0)?)
        }
    }
}
