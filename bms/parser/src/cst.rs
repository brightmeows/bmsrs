//! 从 rowan CST 构建 `Bms`。

use bms_syntax::{BranchSelection, LineKind, SourceFile, TextRange};
use bms_values::{BmsToken, BmsTokenizeError, parse_header_line, parse_message_line};

use crate::Bms;

/// CST 解析产生的非致命错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstWarning {
    /// 出错行的文本范围。
    pub range: TextRange,
    /// 分词错误。
    pub error: BmsTokenizeError,
}

/// 从 CST 构建 `Bms` 的结果。
#[derive(Debug, Clone)]
pub struct CstParse {
    /// 语义模型。
    pub bms: Bms,
    /// 控制流选择记录。
    pub selection: BranchSelection,
    /// 分词警告。
    pub warnings: Vec<CstWarning>,
}

impl Bms {
    /// 从 CST 选择分支并构建 `Bms`。
    ///
    /// `rng(max)` 返回 `[1, max]` 内的值；`#SETRANDOM` / `#SETSWITCH`
    /// 不调用它。
    #[must_use]
    pub fn from_cst(file: &SourceFile, rng: impl FnMut(u64) -> u64) -> CstParse {
        let (lines, selection) = file.select_lines(rng);
        let mut tokens = Vec::new();
        let mut warnings = Vec::new();

        for line in &lines {
            if line.is_directive() {
                continue;
            }
            if !matches!(line.kind(), LineKind::Header | LineKind::Message) {
                continue;
            }
            let Some(content) = line.content_text() else {
                continue;
            };
            match tokenize_line(&content) {
                Ok(Some(token)) => tokens.push(token),
                Ok(None) => {}
                Err(error) => warnings.push(CstWarning {
                    range: line.range(),
                    error,
                }),
            }
        }

        CstParse {
            bms: Self::from_flat_tokens(tokens),
            selection,
            warnings,
        }
    }
}

/// 把一行内容解析为 token。
fn tokenize_line(content: &str) -> Result<Option<BmsToken>, BmsTokenizeError> {
    if let Some(message) = parse_message_line(content)? {
        return Ok(Some(BmsToken::Message(message)));
    }
    Ok(parse_header_line(content, &['#', '%'])?.map(BmsToken::Header))
}
