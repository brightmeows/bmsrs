//! 无损文档：构建、编辑与写回。

use std::fmt;
use std::num::NonZeroUsize;

use bms_control_flow::{ControlFlowError, FlowDoc, FlowRow};
use bms_tokenizer::{BmsHeader, BmsToken, BmsTokenizeError};

use crate::classify::{ScanState, classify_line, classify_str, scan_line};
use crate::source::split_lines;
use crate::{
    DetectedEncoding, DetectionMethod, Line, LineEnding, LineError, LineKind, LineRef,
    SourceEncoding, UnmappableCharacter,
};

/// 文档构建选项。
#[derive(Debug, Clone)]
pub struct DocumentOptions {
    /// 显式指定的编码；`None` 表示自动检测。
    pub encoding: Option<SourceEncoding>,
    /// 允许的头部命令前缀字符，默认 `['#', '%']`。
    pub header_prefixes: Vec<char>,
}

impl Default for DocumentOptions {
    fn default() -> Self {
        Self {
            encoding: None,
            header_prefixes: vec!['#', '%'],
        }
    }
}

/// 一份 BMS 无损文档。
///
/// 文档持有原始字节与编码。未被编辑的行在 [`render`](Self::render) 时
/// 直接切片，不经过解码与重新编码，因此注释、空行、缩进、未知命令与
/// 原始大小写都保持不变。
///
/// # 行编辑
///
/// - [`replace_line`](Self::replace_line)：整行替换，行内 trivia 随旧行丢弃。
/// - [`replace_content`](Self::replace_content)：只替换内容段，前后缀按
///   原始字节保留；对已整体替换的行退化为整行替换。
/// - [`insert_line`](Self::insert_line)：新行继承相邻行的行尾终止符。
/// - [`remove_line`](Self::remove_line)：删除整行。
///
/// 编辑内容在原编码下无法表示时返回 [`EditError::Unmappable`]，文档不变。
/// 新行内容解析失败时仍会被接受，分类为 [`LineKind::Error`]。
pub struct Document {
    /// 原始字节。
    bytes: Vec<u8>,
    /// 采用的编码。
    encoding: SourceEncoding,
    /// BOM 字节长度。
    bom_len: usize,
    /// 编码检测结果。
    detection: DetectedEncoding,
    /// 允许的头部命令前缀。
    prefixes: Vec<char>,
    /// 行列表。
    lines: Vec<Line>,
}

impl fmt::Debug for Document {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Document")
            .field("bytes", &self.bytes.len())
            .field("encoding", &self.encoding)
            .field("bom_len", &self.bom_len)
            .field("detection", &self.detection)
            .field("prefixes", &self.prefixes)
            .field("lines", &self.lines)
            .finish()
    }
}

impl Document {
    /// 自动检测编码，从字节构建文档。
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bytes_with(bytes, &DocumentOptions::default())
    }

    /// 从字节与选项构建文档。
    #[must_use]
    pub fn from_bytes_with(bytes: Vec<u8>, options: &DocumentOptions) -> Self {
        let detection = options.encoding.map_or_else(
            || SourceEncoding::detect(&bytes),
            |encoding| DetectedEncoding {
                encoding,
                bom_len: encoding.bom_len(&bytes),
                declared: None,
                method: DetectionMethod::Explicit,
            },
        );
        let encoding = detection.encoding;
        let bom_len = detection.bom_len;
        let prefixes = options.header_prefixes.clone();

        let body = bytes.get(bom_len..).unwrap_or(&[]);
        let utf16 = encoding.utf16_little_endian();
        let mut state = ScanState::default();
        let mut lines = Vec::new();

        for span in split_lines(body, encoding) {
            let line_body = body.get(span.body.clone()).unwrap_or(&[]);
            let scanned = scan_line(line_body, utf16, &mut state);
            let kind = classify_line(line_body, &scanned, encoding, &prefixes);
            lines.push(Line {
                span: bom_len + span.range.start..bom_len + span.range.end,
                terminator: span.terminator,
                content: scanned
                    .content
                    .map(|range| bom_len + range.start..bom_len + range.end),
                kind,
                replacement: None,
            });
        }

        Self {
            bytes,
            encoding,
            bom_len,
            detection,
            prefixes,
            lines,
        }
    }

    /// 从 UTF-8 文本构建文档。
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let options = DocumentOptions {
            encoding: Some(SourceEncoding::UTF8),
            header_prefixes: DocumentOptions::default().header_prefixes,
        };
        Self::from_bytes_with(text.as_bytes().to_vec(), &options)
    }

    /// 返回文档采用的编码。
    #[must_use]
    pub const fn encoding(&self) -> SourceEncoding {
        self.encoding
    }

    /// 返回编码检测结果。
    #[must_use]
    pub const fn detection(&self) -> &DetectedEncoding {
        &self.detection
    }

    /// 返回 BOM 字节。
    #[must_use]
    pub fn bom(&self) -> &[u8] {
        self.bytes.get(..self.bom_len).unwrap_or(&[])
    }

    /// 返回行列表。
    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// 返回行数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.lines.len()
    }

    /// 返回文档是否没有行。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// 返回指定行。
    #[must_use]
    pub fn line(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    /// 返回指定行的文本（不含行尾终止符）。
    #[must_use]
    pub fn line_text(&self, index: usize) -> Option<String> {
        let line = self.lines.get(index)?;
        let bytes = self.line_bytes(index);
        let body_len = bytes
            .len()
            .saturating_sub(line.terminator.byte_len(self.encoding));
        let body = bytes.get(..body_len)?;
        Some(self.encoding.decode(body).0.into_owned())
    }

    /// 整行替换。
    ///
    /// 旧行的缩进与行内注释随之丢弃；行尾终止符保留。
    ///
    /// # Errors
    ///
    /// 行索引越界，或 `text` 在原编码下无法表示时返回 [`EditError`]。
    pub fn replace_line(&mut self, index: usize, text: &str) -> Result<(), EditError> {
        let Some(line) = self.lines.get(index) else {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        };
        let terminator = line.terminator;
        let mut bytes = self.encoding.encode(text)?;
        terminator.append_to(&mut bytes, self.encoding);
        let kind = self.classify_text(text);

        if let Some(target) = self.lines.get_mut(index) {
            target.replacement = Some(bytes);
            target.content = None;
            target.kind = kind;
        }
        Ok(())
    }

    /// 只替换内容段，保留前缀与后缀 trivia 的原始字节。
    ///
    /// 对已整体替换（或没有内容段）的行，退化为 [`replace_line`](Self::replace_line)。
    ///
    /// # Errors
    ///
    /// 行索引越界，或 `text` 在原编码下无法表示时返回 [`EditError`]。
    pub fn replace_content(&mut self, index: usize, text: &str) -> Result<(), EditError> {
        let Some(line) = self.lines.get(index) else {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        };
        if line.replacement.is_some() {
            return self.replace_line(index, text);
        }
        let Some(content) = line.content.clone() else {
            return self.replace_line(index, text);
        };
        let span = line.span.clone();
        let terminator = line.terminator;
        let body_end = span.end.saturating_sub(terminator.byte_len(self.encoding));
        let encoded = self.encoding.encode(text)?;

        let mut bytes = Vec::with_capacity(span.len().saturating_add(encoded.len()));
        bytes.extend_from_slice(self.bytes.get(span.start..content.start).unwrap_or(&[]));
        bytes.extend_from_slice(&encoded);
        bytes.extend_from_slice(self.bytes.get(content.end..body_end).unwrap_or(&[]));
        terminator.append_to(&mut bytes, self.encoding);
        let kind = self.classify_text(text);

        if let Some(target) = self.lines.get_mut(index) {
            target.replacement = Some(bytes);
            target.kind = kind;
        }
        Ok(())
    }

    /// 在 `index` 之前插入一行。
    ///
    /// 新行继承相邻行的行尾终止符；在无终止符的末行之后插入时，会先给
    /// 末行补上 `LF`，避免两行内容粘连。
    ///
    /// # Errors
    ///
    /// 行索引大于行数，或 `text` 在原编码下无法表示时返回 [`EditError`]。
    pub fn insert_line(&mut self, index: usize, text: &str) -> Result<(), EditError> {
        if index > self.lines.len() {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        }

        let inherited = index
            .checked_sub(1)
            .and_then(|previous| self.lines.get(previous))
            .map(Line::terminator)
            .or_else(|| self.lines.get(index).map(Line::terminator));
        let terminator = match inherited {
            Some(LineEnding::None) | None => LineEnding::Lf,
            Some(other) => other,
        };

        if let Some(previous) = index.checked_sub(1)
            && self
                .lines
                .get(previous)
                .is_some_and(|line| line.terminator == LineEnding::None)
        {
            let mut bytes = self.line_bytes(previous).to_vec();
            LineEnding::Lf.append_to(&mut bytes, self.encoding);
            if let Some(target) = self.lines.get_mut(previous) {
                target.replacement = Some(bytes);
                target.terminator = LineEnding::Lf;
            }
        }

        let mut bytes = self.encoding.encode(text)?;
        terminator.append_to(&mut bytes, self.encoding);
        let kind = self.classify_text(text);
        self.lines.insert(
            index,
            Line {
                span: 0..0,
                terminator,
                content: None,
                kind,
                replacement: Some(bytes),
            },
        );
        Ok(())
    }

    /// 删除指定行并返回它。
    ///
    /// # Errors
    ///
    /// 行索引越界时返回 [`EditError::OutOfBounds`]。
    pub fn remove_line(&mut self, index: usize) -> Result<Line, EditError> {
        if index >= self.lines.len() {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        }
        Ok(self.lines.remove(index))
    }

    /// 输出完整文件字节。
    ///
    /// 未编辑的行原样切片，编辑过的行输出其替换字节。
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.bytes.len());
        output.extend_from_slice(self.bom());
        for index in 0..self.lines.len() {
            output.extend_from_slice(self.line_bytes(index));
        }
        output
    }

    /// 迭代所有头部与通道 token。
    ///
    /// 空白、注释、文本与错误行被跳过。
    pub fn tokens(&self) -> impl Iterator<Item = (NonZeroUsize, BmsToken)> + '_ {
        self.lines.iter().enumerate().filter_map(|(index, line)| {
            line.kind
                .to_token()
                .map(|token| (line_number(index), token))
        })
    }

    /// 迭代所有 token 解析结果。
    ///
    /// 与 [`tokens`](Self::tokens) 的区别是：解析失败的行以
    /// `Err(BmsTokenizeError)` 出现。解码失败的行被跳过。
    pub fn token_results(
        &self,
    ) -> impl Iterator<Item = (NonZeroUsize, Result<BmsToken, BmsTokenizeError>)> + '_ {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| match &line.kind {
                LineKind::Error(LineError::Tokenize(error)) => {
                    Some((line_number(index), Err(error.clone())))
                }
                other => other
                    .to_token()
                    .map(|token| (line_number(index), Ok(token))),
            })
    }

    /// 构建行导向的控制流树。
    ///
    /// 每一行都作为一个条目进入树：控制流命令行成为骨架，其余行（含
    /// 空白、注释与错误行）进入载荷片段。因此载荷行区间覆盖原文的连续
    /// 行，[`select_payloads`](bms_control_flow::FlowDoc::select_payloads)
    /// 的结果可以直接交给 [`render_lines`](Self::render_lines) 做字节保真
    /// 渲染。
    ///
    /// # Errors
    ///
    /// 控制流嵌套错误时返回 [`ControlFlowError`]。
    pub fn flow(&self) -> Result<FlowDoc<LineRef>, ControlFlowError> {
        let rows = self.lines.iter().enumerate().map(|(index, line)| {
            let number = line_number(index);
            match &line.kind {
                LineKind::Header(BmsHeader::ControlFlow(cf)) => {
                    (number, FlowRow::ControlFlow(cf.clone()))
                }
                _ => (number, FlowRow::Payload(())),
            }
        });
        FlowDoc::from_rows(rows)
    }

    /// 按行区间渲染扁平输出。
    ///
    /// 输出包含 BOM（若有），随后按给定顺序拼接各行。未编辑行取原始
    /// 字节，因此结果中对应行的字节与原文一致。
    #[must_use]
    pub fn render_lines(&self, ranges: &[LineRef]) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(self.bom());
        for range in ranges {
            for number in range.start.get()..=range.end.get() {
                output.extend_from_slice(self.line_bytes(number.saturating_sub(1)));
            }
        }
        output
    }

    /// 返回指定行的当前字节（优先使用编辑后的替换字节）。
    fn line_bytes(&self, index: usize) -> &[u8] {
        self.lines.get(index).map_or(&[], |line| {
            line.replacement
                .as_deref()
                .unwrap_or_else(|| self.bytes.get(line.span.clone()).unwrap_or_default())
        })
    }

    /// 对新行文本做分类。
    fn classify_text(&self, text: &str) -> LineKind {
        let mut state = ScanState::default();
        let scanned = scan_line(text.as_bytes(), None, &mut state);
        classify_str(text, &scanned, &self.prefixes)
    }
}

/// 行编辑错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// 行索引越界。
    #[error("line index {index} out of bounds (len {len})")]
    OutOfBounds {
        /// 请求的索引。
        index: usize,
        /// 文档行数。
        len: usize,
    },
    /// 文本在原编码下无法表示。
    #[error(transparent)]
    Unmappable(#[from] UnmappableCharacter),
}

/// 把 0 起索引转换为 1 起的行号。
fn line_number(index: usize) -> NonZeroUsize {
    NonZeroUsize::new(index.saturating_add(1)).unwrap_or(NonZeroUsize::MIN)
}
