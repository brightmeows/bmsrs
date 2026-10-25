//! 错误类型与退出码。

use std::path::PathBuf;

/// `bmsedit` 的统一错误。
#[derive(Debug, thiserror::Error)]
pub enum BmsEditError {
    /// 文件读写失败。
    #[error("io error on {path}: {source}")]
    Io {
        /// 出错的文件路径。
        path: PathBuf,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },
    /// 文件解析失败（BMS 或 bmson）。
    #[error("parse failed: {0}")]
    Parse(String),
    /// 用法或目标定位错误——大模型可调整参数后重试。
    #[error("{0}")]
    Usage(String),
    /// 编辑结果未通过验证——文件未被修改。
    #[error("verification failed: {0}")]
    Verify(String),
    /// 当前文件形态不支持该操作。
    #[error("unsupported: {0}")]
    Unsupported(String),
}

impl BmsEditError {
    /// 进程退出码：`0` 成功；`1` 用法错误（可调整重试）；`2` 硬错误。
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 1,
            Self::Io { .. } | Self::Parse(_) | Self::Verify(_) | Self::Unsupported(_) => 2,
        }
    }

    /// 机器可读的错误码。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Parse(_) => "parse",
            Self::Usage(_) => "usage",
            Self::Verify(_) => "verify",
            Self::Unsupported(_) => "unsupported",
        }
    }
}
