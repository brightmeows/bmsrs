//! 分析入口。

use std::collections::BTreeMap;

use bms_parser::{BmsChannel, parse_header_line, parse_message_line};
use bms_syntax::{Line, LineKind, Parse, SourceFile};

use crate::diagnostic::{AnalysisDiagnostic, Severity};
use crate::index::{SymbolDefinition, SymbolIndex, SymbolKind, SymbolReference};

/// 一次分析结果。
#[derive(Debug, Clone)]
pub struct Analysis {
    /// CST 与语法诊断。
    parse: Parse,
    /// 诊断列表。
    diagnostics: Vec<AnalysisDiagnostic>,
    /// 符号索引。
    index: SymbolIndex,
}

impl Analysis {
    /// 分析 BMS 文本。
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let parse = SourceFile::parse(text);
        let source = parse.source_file();
        let mut diagnostics: Vec<_> = parse
            .errors()
            .iter()
            .map(|error| {
                let severity = match error.kind() {
                    bms_syntax::DiagnosticKind::Syntax => Severity::Error,
                    bms_syntax::DiagnosticKind::ControlFlow => Severity::Warning,
                };
                AnalysisDiagnostic::new(error.range(), severity, error.message(), None)
            })
            .collect();
        let mut index = SymbolIndex::default();
        collect_lines(&source, &mut index, &mut diagnostics);
        add_duplicate_diagnostics(&index, &mut diagnostics);
        add_missing_reference_diagnostics(&index, &mut diagnostics);

        Self {
            parse,
            diagnostics,
            index,
        }
    }

    /// 返回文档根节点。
    #[must_use]
    pub fn source_file(&self) -> SourceFile {
        self.parse.source_file()
    }

    /// 返回新建的红色根节点。
    #[must_use]
    pub fn syntax(&self) -> bms_syntax::SyntaxNode {
        self.parse.syntax()
    }

    /// 返回诊断。
    #[must_use]
    pub fn diagnostics(&self) -> &[AnalysisDiagnostic] {
        &self.diagnostics
    }

    /// 返回符号索引。
    #[must_use]
    pub const fn index(&self) -> &SymbolIndex {
        &self.index
    }
}

/// 收集定义与引用。
fn collect_lines(
    source: &SourceFile,
    index: &mut SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    for line in source.lines() {
        match line.kind() {
            LineKind::Header if !line.is_directive() => {
                collect_header(&line, index, diagnostics);
            }
            LineKind::Message => collect_message(&line, index, diagnostics),
            _ => {}
        }
    }
}

/// 处理头部行。
fn collect_header(line: &Line, index: &mut SymbolIndex, diagnostics: &mut Vec<AnalysisDiagnostic>) {
    let Some(command) = line.command_name() else {
        return;
    };
    let Some(content) = line.content_text() else {
        return;
    };
    match parse_header_line(&content, &['#', '%']) {
        Ok(Some(_header)) => {
            if let Some((kind, id)) = definition_kind(command.text()) {
                index.push_definition(SymbolDefinition::new(
                    kind,
                    id,
                    line.range(),
                    command.text_range(),
                ));
            }
        }
        Ok(None) => {}
        Err(error) => diagnostics.push(AnalysisDiagnostic::new(
            line.range(),
            Severity::Error,
            error.to_string(),
            Some("tokenize"),
        )),
    }
}

/// 处理通道行。
fn collect_message(
    line: &Line,
    index: &mut SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let Some(content) = line.content_text() else {
        return;
    };
    let message = match parse_message_line(&content) {
        Ok(Some(message)) => message,
        Ok(None) => return,
        Err(error) => {
            diagnostics.push(AnalysisDiagnostic::new(
                line.range(),
                Severity::Error,
                error.to_string(),
                Some("tokenize"),
            ));
            return;
        }
    };
    let Some(kind) = reference_kind(message.channel()) else {
        return;
    };
    for chunk in line.body_chunks() {
        let id = chunk.text().to_ascii_uppercase();
        if id == "00" {
            continue;
        }
        index.push_reference(SymbolReference::new(kind, id, chunk.text_range()));
    }
}

/// 从命令名识别定义种类与 ID。
fn definition_kind(command: &str) -> Option<(SymbolKind, String)> {
    let upper = command.to_ascii_uppercase();
    let prefixes = [
        ("EXWAV", SymbolKind::Wav),
        ("WAV", SymbolKind::Wav),
        ("EXBMP", SymbolKind::Bmp),
        ("BMP", SymbolKind::Bmp),
        ("@BGA", SymbolKind::Bga),
        ("SWBGA", SymbolKind::Bga),
        ("ARGB", SymbolKind::Bga),
        ("BGA", SymbolKind::Bga),
        ("EXBPM", SymbolKind::Bpm),
        ("BPM", SymbolKind::Bpm),
        ("STOP", SymbolKind::Stop),
        ("SCROLL", SymbolKind::Scroll),
        ("SPEED", SymbolKind::Speed),
        ("EXRANK", SymbolKind::ExRank),
        ("CHANGEOPTION", SymbolKind::ChangeOption),
        ("SEEK", SymbolKind::Seek),
        ("TEXT", SymbolKind::Text),
        ("SONG", SymbolKind::Text),
    ];

    prefixes.iter().find_map(|(prefix, kind)| {
        let id = upper.strip_prefix(prefix)?;
        ((1..=2).contains(&id.len()) && id.chars().all(|ch| ch.is_ascii_alphanumeric()))
            .then(|| (*kind, id.to_owned()))
    })
}

/// 从通道识别引用种类。
const fn reference_kind(channel: BmsChannel) -> Option<SymbolKind> {
    match channel {
        BmsChannel::Bgm | BmsChannel::Note(_) => Some(SymbolKind::Wav),
        BmsChannel::BgaBase
        | BmsChannel::BgaPoor
        | BmsChannel::BgaLayer
        | BmsChannel::BgaLayer2 => Some(SymbolKind::Bga),
        BmsChannel::ExtendedBpm => Some(SymbolKind::Bpm),
        BmsChannel::Stop => Some(SymbolKind::Stop),
        BmsChannel::Scroll => Some(SymbolKind::Scroll),
        BmsChannel::Speed => Some(SymbolKind::Speed),
        BmsChannel::Text => Some(SymbolKind::Text),
        BmsChannel::Option => Some(SymbolKind::ChangeOption),
        BmsChannel::Seek => Some(SymbolKind::Seek),
        BmsChannel::MeasureLength
        | BmsChannel::BpmChange
        | BmsChannel::BgaBaseOpacity
        | BmsChannel::BgaLayerOpacity
        | BmsChannel::BgaLayer2Opacity
        | BmsChannel::BgaPoorOpacity
        | BmsChannel::BgmVolume
        | BmsChannel::KeyVolume
        | BmsChannel::Judge
        | BmsChannel::BgaArgbBase
        | BmsChannel::BgaArgbLayer
        | BmsChannel::BgaArgbLayer2
        | BmsChannel::BgaArgbPoor
        | BmsChannel::BgaKeyBound
        | BmsChannel::Unknown(_) => None,
    }
}

/// 添加重复定义诊断。
fn add_duplicate_diagnostics(index: &SymbolIndex, diagnostics: &mut Vec<AnalysisDiagnostic>) {
    let mut groups: BTreeMap<(SymbolKind, String), Vec<usize>> = BTreeMap::new();
    for (position, definition) in index.definitions().iter().enumerate() {
        groups
            .entry((definition.kind(), definition.id().to_owned()))
            .or_default()
            .push(position);
    }
    for ((kind, id), positions) in groups {
        if positions.len() > 1 {
            for position in positions {
                if let Some(definition) = index.definitions().get(position) {
                    diagnostics.push(AnalysisDiagnostic::new(
                        definition.range(),
                        Severity::Warning,
                        format!("重复定义 {} {}", kind.label(), id),
                        Some("duplicate-definition"),
                    ));
                }
            }
        }
    }
}

/// 添加缺失引用诊断。
fn add_missing_reference_diagnostics(
    index: &SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    for reference in index.references() {
        let found = index.has_definition(reference.kind(), reference.id())
            || (reference.kind() == SymbolKind::Bga
                && index.has_definition(SymbolKind::Bmp, reference.id()));
        if !found {
            diagnostics.push(AnalysisDiagnostic::new(
                reference.range(),
                Severity::Warning,
                format!("未定义 {} {}", reference.kind().label(), reference.id()),
                Some("undefined-reference"),
            ));
        }
    }
}
