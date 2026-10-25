//! 输出层：人话默认，`--json` 结构化。
//!
//! 全部输出经本模块缓冲后一次性写出，集中处理
//! `clippy::print_stdout` / `clippy::print_stderr` 豁免。

use serde::Serialize;

use crate::error::BmsEditError;

/// JSON 成功包络。
#[derive(Serialize)]
struct JsonEnvelope<'a> {
    /// 成功标志。
    ok: bool,
    /// 读取警告。
    warnings: &'a [String],
    /// 命令数据。
    data: &'a serde_json::Value,
}

/// JSON 错误包络。
#[derive(Serialize)]
struct JsonError<'a> {
    /// 失败标志。
    ok: bool,
    /// 错误体。
    error: JsonErrorBody<'a>,
}

/// JSON 错误体。
#[derive(Serialize)]
struct JsonErrorBody<'a> {
    /// 错误码。
    code: &'a str,
    /// 错误消息。
    message: String,
}

/// 输出缓冲。
pub struct Output {
    /// 是否 JSON 模式。
    json: bool,
    /// stdout 行缓冲。
    stdout: Vec<String>,
    /// stderr 行缓冲。
    stderr: Vec<String>,
}

impl Output {
    /// 创建输出缓冲。
    #[must_use]
    pub const fn new(json: bool) -> Self {
        Self {
            json,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    /// 输出命令结果：人话模式用 `human` 文本，JSON 模式序列化 `data`。
    pub fn result(&mut self, human: String, data: &impl Serialize, warnings: &[String]) {
        if self.json {
            let value = serde_json::to_value(data).unwrap_or(serde_json::Value::Null);
            let envelope = JsonEnvelope {
                ok: true,
                warnings,
                data: &value,
            };
            self.stdout
                .push(serde_json::to_string(&envelope).unwrap_or_default());
        } else {
            self.stdout.push(human);
        }
    }

    /// 输出错误（stderr）。
    pub fn error(&mut self, err: &BmsEditError) {
        if self.json {
            let body = JsonError {
                ok: false,
                error: JsonErrorBody {
                    code: err.code(),
                    message: err.to_string(),
                },
            };
            self.stderr
                .push(serde_json::to_string(&body).unwrap_or_default());
        } else {
            self.stderr.push(format!("error[{}]: {err}", err.code()));
        }
    }

    /// 冲刷缓冲到标准流。
    #[expect(clippy::print_stdout, reason = "CLI 的本质输出；集中一处豁免")]
    #[expect(clippy::print_stderr, reason = "错误走 stderr；集中一处豁免")]
    pub fn flush(&mut self) {
        for line in self.stdout.drain(..) {
            println!("{line}");
        }
        for line in self.stderr.drain(..) {
            eprintln!("{line}");
        }
    }
}
