//! Analysis entry point.

use std::collections::BTreeMap;

use bmsrs_bms_syntax::{
    IncrementalParser, ParseOptions, SourceFile, SyntaxLine, SyntaxLineKind, SyntaxParse,
};
use bmsrs_bms_values::{BmsChannel, BmsHeader, BmsMessage, BodyCells};

use crate::diagnostic::{AnalysisDiagnostic, Severity};
use crate::index::{SymbolDefinition, SymbolId, SymbolIndex, SymbolKind, SymbolReference};

/// Result of one analysis.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// Incremental syntax state.
    syntax: IncrementalParser,
    /// Diagnostic list.
    diagnostics: Vec<AnalysisDiagnostic>,
    /// Symbol index.
    index: SymbolIndex,
}

impl Analysis {
    /// Analyze BMS text.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::from_text_with(text, &ParseOptions::default())
    }

    /// Analyze BMS text with parse options.
    #[must_use]
    pub fn from_text_with(text: &str, options: &ParseOptions) -> Self {
        let syntax = IncrementalParser::new(text, options.clone());
        let (diagnostics, index) = analyze(syntax.parse(), options);
        Self {
            syntax,
            diagnostics,
            index,
        }
    }

    /// Return the analyzed source text.
    #[must_use]
    pub fn text(&self) -> &str {
        self.syntax.text()
    }

    /// Replace the source text and refresh diagnostics and the symbol index.
    pub fn update(&mut self, text: &str) {
        if text == self.syntax.text() {
            return;
        }
        let old_text = self.syntax.text().to_owned();
        let changed_line = single_line_change(&old_text, text);
        self.syntax.update(text);

        if let Some(line) = changed_line {
            let options = self.syntax.options().clone();
            let source = self.syntax.parse().source_file();
            let line_starts = source
                .lines()
                .map(|source_line| source_line.range().start())
                .collect();
            self.index.remove_line(line);
            self.index.set_line_starts(line_starts);
            collect_line_at(
                &source,
                line,
                &options.header_prefixes,
                &mut self.index,
                &mut self.diagnostics,
            );
            self.diagnostics = diagnostics_from(self.syntax.parse(), &options, &self.index);
        } else {
            let (diagnostics, index) = analyze(self.syntax.parse(), self.syntax.options());
            self.diagnostics = diagnostics;
            self.index = index;
        }
    }

    /// Return the document root node.
    #[must_use]
    pub fn source_file(&self) -> SourceFile {
        self.syntax.parse().source_file()
    }

    /// Return a fresh red root node.
    #[must_use]
    pub fn syntax(&self) -> bmsrs_bms_syntax::SyntaxNode {
        self.syntax.parse().syntax()
    }

    /// Return the diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[AnalysisDiagnostic] {
        &self.diagnostics
    }

    /// Return the symbol index.
    #[must_use]
    pub const fn index(&self) -> &SymbolIndex {
        &self.index
    }
}

/// Build diagnostics and the symbol index from one parse result.
fn analyze(parse: &SyntaxParse, options: &ParseOptions) -> (Vec<AnalysisDiagnostic>, SymbolIndex) {
    let source = parse.source_file();
    let mut diagnostics: Vec<_> = parse
        .errors()
        .iter()
        .map(|error| {
            let severity = match error.kind() {
                bmsrs_bms_syntax::DiagnosticKind::Syntax => Severity::Error,
                bmsrs_bms_syntax::DiagnosticKind::ControlFlow => Severity::Warning,
            };
            AnalysisDiagnostic::new(error.range(), severity, error.message(), None)
        })
        .collect();
    let mut index = SymbolIndex::default();
    collect_lines(
        &source,
        &options.header_prefixes,
        &mut index,
        &mut diagnostics,
    );
    let line_starts = source.lines().map(|line| line.range().start()).collect();
    index.set_line_starts(line_starts);
    add_duplicate_diagnostics(&index, &mut diagnostics);
    add_missing_reference_diagnostics(&index, &mut diagnostics);
    (diagnostics, index)
}

/// Collect definitions and references.
fn collect_lines(
    source: &SourceFile,
    prefixes: &[char],
    index: &mut SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    for (line_index, line) in source.lines().enumerate() {
        collect_line(line_index, &line, prefixes, index, diagnostics);
    }
}

/// Collect one line into the index and diagnostics.
fn collect_line(
    line_index: usize,
    line: &SyntaxLine,
    prefixes: &[char],
    index: &mut SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let line_number = u32::try_from(line_index).unwrap_or(u32::MAX);
    match line.kind() {
        SyntaxLineKind::Header if !line.is_directive() => {
            let content_text = line.content_text();
            if let Some(content) = content_text.as_deref()
                && let Ok(Some(message)) = BmsMessage::parse(content)
            {
                let body_start = content
                    .find(':')
                    .map_or(0, |offset| offset.saturating_add(1));
                let base = content_base(line, content).saturating_add(body_start);
                let line_start = usize::from(line.range().start());
                collect_message_refs(
                    message.channel(),
                    &message.body,
                    line_number,
                    line_start,
                    base,
                    index,
                );
            } else {
                collect_header(line_number, line, prefixes, index, diagnostics);
            }
        }
        SyntaxLineKind::Message => collect_message(line_number, line, index, diagnostics),
        _ => {}
    }
}

/// Collect one line by index.
fn collect_line_at(
    source: &SourceFile,
    line_index: u32,
    prefixes: &[char],
    index: &mut SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let Some(index_in_file) = usize::try_from(line_index).ok() else {
        return;
    };
    let Some(line) = source.lines().nth(index_in_file) else {
        return;
    };
    collect_line(index_in_file, &line, prefixes, index, diagnostics);
}

/// Process a header line.
fn collect_header(
    line_index: u32,
    line: &SyntaxLine,
    prefixes: &[char],
    index: &mut SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let Some(command) = line.command_name() else {
        return;
    };
    let Some(content) = line.content_text() else {
        return;
    };
    match BmsHeader::parse(&content, prefixes) {
        Ok(Some(_header)) => {
            if let Some((kind, id)) = definition_kind(command.text())
                && let Some(symbol_id) = SymbolId::new(&id)
            {
                let line_start = usize::from(line.range().start());
                let start = 0;
                let end = u32::try_from(usize::from(line.range().end()).saturating_sub(line_start))
                    .unwrap_or(u32::MAX);
                let name_start = u32::try_from(
                    usize::from(command.text_range().start()).saturating_sub(line_start),
                )
                .unwrap_or(u32::MAX);
                let name_end = u32::try_from(
                    usize::from(command.text_range().end()).saturating_sub(line_start),
                )
                .unwrap_or(u32::MAX);
                index.push_definition(SymbolDefinition::new(
                    kind, symbol_id, line_index, start, end, name_start, name_end,
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

/// Process a channel line.
fn collect_message(
    line_index: u32,
    line: &SyntaxLine,
    index: &mut SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let Some(content) = line.content_text() else {
        return;
    };
    let message = match BmsMessage::parse(&content) {
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
    let body_start = content
        .find(':')
        .map_or(0, |offset| offset.saturating_add(1));
    let base = content_base(line, &content).saturating_add(body_start);
    let line_start = usize::from(line.range().start());
    collect_message_refs(
        message.channel(),
        &message.body,
        line_index,
        line_start,
        base,
        index,
    );
}

/// Collect message references for a channel.
fn collect_message_refs(
    channel: BmsChannel,
    body: &str,
    line_index: u32,
    line_start: usize,
    base: usize,
    index: &mut SymbolIndex,
) {
    let Some(kind) = reference_kind(channel) else {
        return;
    };
    for cell in BodyCells::including_zero(body) {
        let id = cell.text.to_ascii_uppercase();
        if id == "00" {
            continue;
        }
        let start = u32::try_from(
            base.saturating_add(cell.range.start)
                .saturating_sub(line_start),
        )
        .unwrap_or(u32::MAX);
        let end = u32::try_from(
            base.saturating_add(cell.range.end)
                .saturating_sub(line_start),
        )
        .unwrap_or(u32::MAX);
        if let Some(symbol_id) = SymbolId::new(&id) {
            index.push_reference(SymbolReference::new(
                kind, symbol_id, line_index, start, end,
            ));
        }
    }
}

/// Return the start offset of the content span within the whole line text.
fn content_base(line: &SyntaxLine, content: &str) -> usize {
    let line_text = line.syntax().text().to_string();
    let content_offset = line_text.find(content).unwrap_or(0);
    usize::from(line.range().start()).saturating_add(content_offset)
}

/// Identify the definition kind and ID from a command name.
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

/// Identify the reference kind from a channel.
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

/// Add duplicate-definition diagnostics.
fn add_duplicate_diagnostics(index: &SymbolIndex, diagnostics: &mut Vec<AnalysisDiagnostic>) {
    let mut groups: BTreeMap<(SymbolKind, String), Vec<usize>> = BTreeMap::new();
    let definitions: Vec<_> = index.definitions().collect();
    for (position, definition) in definitions.iter().enumerate() {
        groups
            .entry((definition.kind(), definition.id().to_owned()))
            .or_default()
            .push(position);
    }
    for ((kind, id), positions) in groups {
        if positions.len() > 1 {
            for position in positions {
                if let Some(definition) = definitions.get(position)
                    && let Some(range) = index.range_of_definition(definition)
                {
                    diagnostics.push(AnalysisDiagnostic::new(
                        range,
                        Severity::Warning,
                        format!("duplicate definition of {} {}", kind.label(), id),
                        Some("duplicate-definition"),
                    ));
                }
            }
        }
    }
}

/// Add missing-reference diagnostics.
fn add_missing_reference_diagnostics(
    index: &SymbolIndex,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    for reference in index.references() {
        let found = index.has_definition(reference.kind(), reference.id())
            || (reference.kind() == SymbolKind::Bga
                && index.has_definition(SymbolKind::Bmp, reference.id()));
        if !found && let Some(range) = index.range_of_reference(reference) {
            diagnostics.push(AnalysisDiagnostic::new(
                range,
                Severity::Warning,
                format!("undefined {} {}", reference.kind().label(), reference.id()),
                Some("undefined-reference"),
            ));
        }
    }
}

/// Return the changed line number when the edit is confined to one line.
fn single_line_change(old: &str, new: &str) -> Option<u32> {
    if old.lines().count() != new.lines().count() {
        return None;
    }
    let prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    let max_suffix = old.len().min(new.len()).saturating_sub(prefix);
    let suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(max_suffix)
        .take_while(|(left, right)| left == right)
        .count();
    let old_changed_end = old.len().saturating_sub(suffix);
    let new_changed_end = new.len().saturating_sub(suffix);
    let old_changed = old.get(prefix..old_changed_end)?;
    let new_changed = new.get(prefix..new_changed_end)?;
    if old_changed.contains('\n') || new_changed.contains('\n') {
        return None;
    }
    let line = old
        .get(..prefix)?
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    u32::try_from(line).ok()
}

/// Build diagnostics from a parse result and an already-built index.
fn diagnostics_from(
    parse: &SyntaxParse,
    options: &ParseOptions,
    index: &SymbolIndex,
) -> Vec<AnalysisDiagnostic> {
    let source = parse.source_file();
    let mut diagnostics: Vec<_> = parse
        .errors()
        .iter()
        .map(|error| {
            let severity = match error.kind() {
                bmsrs_bms_syntax::DiagnosticKind::Syntax => Severity::Error,
                bmsrs_bms_syntax::DiagnosticKind::ControlFlow => Severity::Warning,
            };
            AnalysisDiagnostic::new(error.range(), severity, error.message(), None)
        })
        .collect();
    let mut scratch = SymbolIndex::default();
    collect_lines(
        &source,
        &options.header_prefixes,
        &mut scratch,
        &mut diagnostics,
    );
    add_duplicate_diagnostics(index, &mut diagnostics);
    add_missing_reference_diagnostics(index, &mut diagnostics);
    diagnostics.sort_by_key(|diagnostic| (diagnostic.range().start(), diagnostic.range().end()));
    diagnostics
}
