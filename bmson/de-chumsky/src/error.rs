//! bmson 反序列化管道的错误类型。

/// BMSON 解析与反序列化过程中可能出现的错误。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BmsonDeError {
    /// 致命的 JSON 解析错误（chumsky 未产生任何输出）。
    ///
    /// 包含来自解析器的可读诊断信息。
    #[error("JSON parse error(s):\n{0}")]
    JsonParse(String),

    /// bmson 版本字符串缺失或无法识别。
    #[error("{0}")]
    UnknownVersion(String),

    /// 从 [`serde_json::Value`] 反序列化版本特定类型失败
    /// （例如缺少必填字段、类型不匹配）。
    #[error("Failed to deserialize {version} bmson: {message}")]
    Deserialize {
        /// 可读的版本标识符（例如 `"v2.0.0"`、`"v1.0.0"`）。
        version: &'static str,
        /// 底层错误描述。
        message: String,
    },

    /// 从旧版 v0.2.1 格式转换为统一 v2 格式失败
    /// （例如 `init_bpm` 非法）。
    #[error("V0 conversion error: {0}")]
    V0Conversion(String),
}

impl From<bmson_def::BmsonError> for BmsonDeError {
    fn from(e: bmson_def::BmsonError) -> Self {
        match e {
            bmson_def::BmsonError::UnknownVersion(v) => Self::UnknownVersion(v),
            bmson_def::BmsonError::V0Conversion(v) => Self::V0Conversion(v),
            _ => Self::UnknownVersion(e.to_string()),
        }
    }
}

impl From<bmson_def::v0::TryFromV0Error> for BmsonDeError {
    fn from(e: bmson_def::v0::TryFromV0Error) -> Self {
        Self::V0Conversion(e.message)
    }
}
