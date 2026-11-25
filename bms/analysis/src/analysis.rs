//! Analysis entry point.

use std::collections::BTreeMap;

use bmsrs_bms_syntax::{
    IncrementalParser, ParseOptions, SourceFile, SyntaxLine, SyntaxLineKind, SyntaxParse,
};
use bmsrs_bms_values::{
    BmsChannel, BmsHeader, BmsHeaderConditional, BmsHeaderGameplay, BmsMessage, BodyCells,
};

use bmsrs_pipeline::DiagnosticCode;

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
            AnalysisDiagnostic::new(
                error.range(),
                error.severity(),
                error.message(),
                Some(error.code()),
            )
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

/// Check `KaleidBeat` conditional-block structure.
///
/// `KaleidBeat` extension: the block syntax is not defined by the BMS
/// specification or beatoraja. The analyzer reports structural mistakes; it
/// does not evaluate conditions.
#[expect(
    clippy::too_many_lines,
    reason = "single state machine over conditional directives; splitting hides the ordering rules"
)]
fn check_conditional(
    source: &SourceFile,
    prefixes: &[char],
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let mut depth = 0usize;
    let mut branch_count = 0usize;
    let mut saw_default = false;
    let mut block_start = None;

    for line in source.lines() {
        let Some(content) = line.content_text() else {
            continue;
        };
        let Ok(Some(header)) = BmsHeader::parse(&content, prefixes) else {
            continue;
        };
        let BmsHeader::Conditional(command) = header else {
            if depth > 0 && line.kind() == SyntaxLineKind::Header {
                diagnostics.push(AnalysisDiagnostic::new(
                    line.range(),
                    Severity::Warning,
                    "header inside a #CONDITIONAL branch; headers are global in KaleidBeat",
                    Some(DiagnosticCode::Conditional),
                ));
            }
            continue;
        };
        match command {
            BmsHeaderConditional::Conditional(position) => {
                if depth > 0 {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "nested #CONDITIONAL block; the outer block is not closed",
                        Some(DiagnosticCode::Conditional),
                    ));
                }
                depth = 1;
                branch_count = 0;
                saw_default = false;
                block_start = Some(line.range());
                if !position.is_finite() || !(0.0..=999.999_999).contains(&position) {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "conditional evaluation position is out of range",
                        Some(DiagnosticCode::Conditional),
                    ));
                }
            }
            BmsHeaderConditional::When(_) => {
                if depth == 0 {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#WHEN outside a #CONDITIONAL block",
                        Some(DiagnosticCode::Conditional),
                    ));
                } else if saw_default {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#WHEN after #DEFAULT",
                        Some(DiagnosticCode::Conditional),
                    ));
                } else {
                    branch_count += 1;
                }
            }
            BmsHeaderConditional::ElseWhen(_) => {
                if depth == 0 {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#ELSEWHEN outside a #CONDITIONAL block",
                        Some(DiagnosticCode::Conditional),
                    ));
                } else if branch_count == 0 {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#ELSEWHEN without a preceding #WHEN",
                        Some(DiagnosticCode::Conditional),
                    ));
                } else if saw_default {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#ELSEWHEN after #DEFAULT",
                        Some(DiagnosticCode::Conditional),
                    ));
                }
            }
            BmsHeaderConditional::Default => {
                if depth == 0 {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#DEFAULT outside a #CONDITIONAL block",
                        Some(DiagnosticCode::Conditional),
                    ));
                } else if saw_default {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "duplicate #DEFAULT in a #CONDITIONAL block",
                        Some(DiagnosticCode::Conditional),
                    ));
                } else if branch_count == 0 {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#DEFAULT without a preceding #WHEN",
                        Some(DiagnosticCode::Conditional),
                    ));
                }
                saw_default = true;
            }
            BmsHeaderConditional::EndConditional => {
                if depth == 0 {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#ENDCONDITIONAL without a matching #CONDITIONAL",
                        Some(DiagnosticCode::Conditional),
                    ));
                } else if !saw_default {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Warning,
                        "#CONDITIONAL block is missing #DEFAULT",
                        Some(DiagnosticCode::Conditional),
                    ));
                }
                depth = 0;
                branch_count = 0;
                saw_default = false;
                block_start = None;
            }
        }
    }

    if let Some(range) = block_start {
        diagnostics.push(AnalysisDiagnostic::new(
            range,
            Severity::Warning,
            "#CONDITIONAL block is not closed by #ENDCONDITIONAL",
            Some(DiagnosticCode::Conditional),
        ));
    }
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
    check_conditional(source, prefixes, diagnostics);
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
            if let Some(command) = line.command_name()
                && let Some(name) = extension_command(command.text())
            {
                diagnostics.push(AnalysisDiagnostic::new(
                    command.text_range(),
                    Severity::Information,
                    format!(
                        "`KaleidBeat` extension command `#{name}`: preserved verbatim, semantics not modeled"
                    ),
                    Some(DiagnosticCode::Extension),
                ));
            }
            let content_text = line.content_text();
            if let Some(content) = content_text.as_deref()
                && let Ok(Some(message)) = BmsMessage::parse(content)
            {
                if let Some(code) = extension_channel_code(message.channel()) {
                    diagnostics.push(AnalysisDiagnostic::new(
                        line.range(),
                        Severity::Information,
                        format!(
                            "`KaleidBeat` extension channel {code}: kept in the dialect sidecar"
                        ),
                        Some(DiagnosticCode::Extension),
                    ));
                    return;
                }
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
        Ok(Some(header)) => {
            if let Some((kind, id)) =
                marker_definition(&header).or_else(|| definition_kind(command.text()))
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
            Some(DiagnosticCode::Tokenize),
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
                Some(DiagnosticCode::Tokenize),
            ));
            return;
        }
    };
    if let Some(code) = extension_channel_code(message.channel()) {
        diagnostics.push(AnalysisDiagnostic::new(
            line.range(),
            Severity::Information,
            format!("`KaleidBeat` extension channel {code}: kept in the dialect sidecar"),
            Some(DiagnosticCode::Extension),
        ));
        return;
    }
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

/// Header commands from the `KaleidBeat` chart dialect that this repository
/// recognizes but does not model.
///
/// The lossless pipeline preserves such lines verbatim; the diagnostic only
/// marks their presence for editors. Command semantics live in
/// [`Docs::header`](crate::Docs::header); this list decides recognition.
///
/// The dialect belongs to the `KaleidBeat` player
/// (<https://github.com/curecoma/KaleidBeat-releases>). Its format manual ships
/// inside the release archive as `KaleidBeat/documents/format-header.html`;
/// the reference implementation's `docs/kaleid.md` is a secondary contract.
///
/// Commands already modeled by `bmsrs-bms-values` are intentionally absent:
/// they no longer produce a "not modeled" diagnostic. Only the conditional
/// family and the `#BMD` trigger remain here.
const EXTENSION_COMMANDS: &[&str] = &["BMD"];

/// Return the canonical name when `command` is a recognized dialect-extension
/// command.
fn extension_command(command: &str) -> Option<&'static str> {
    let upper = command.to_ascii_uppercase();
    EXTENSION_COMMANDS
        .iter()
        .find(|candidate| **candidate == upper)
        .copied()
}

/// Return the channel code when `channel` is an unambiguous dialect-extension
/// channel.
///
/// | Code | Dialect meaning |
/// |------|-----------------|
/// | `ST` | bar line at the object positions |
/// | `SF` / `SH` | one-shot free zone (1P / 2P) |
/// | `SG` / `SI` | paired free-zone delimiter (1P / 2P) |
/// | `F`-`O` + digit | potion, dummy tap, dummy long, dummy mine, or dummy potion |
///
/// Per-channel summaries live in [`Docs::channel`](crate::Docs::channel).
///
/// `17` / `27` are deliberately not reported: the manual treats them as the
/// legacy free-zone form whose meaning the player infers from the chart's era,
/// and other extensions (such as nanasi's foot pedal) also place objects
/// there, so flagging them would mislabel ordinary charts.
fn extension_channel_code(channel: BmsChannel) -> Option<String> {
    let text = match channel {
        BmsChannel::Note(index) | BmsChannel::Unknown(index) => index.to_string(),
        BmsChannel::Extension(extension) => extension.to_string(),
        _ => return None,
    };
    let code = text.to_ascii_uppercase();
    let is_extension = matches!(code.as_str(), "ST" | "SF" | "SG" | "SH" | "SI")
        || matches!(code.as_bytes(), [b'F'..=b'O', b'1'..=b'9']);
    is_extension.then_some(code)
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

/// Identify a long-note terminator marker definition from the parsed header.
///
/// `#LNOBJ` and the typed markers carry their id as the command **value**, not
/// as a command-name suffix, so the prefix table cannot reach them.
fn marker_definition(header: &BmsHeader) -> Option<(SymbolKind, String)> {
    let BmsHeader::Gameplay(gameplay) = header else {
        return None;
    };
    let (BmsHeaderGameplay::LnObj(id)
    | BmsHeaderGameplay::CnObj(id)
    | BmsHeaderGameplay::HcnObj(id)
    | BmsHeaderGameplay::HlnObj(id)) = gameplay
    else {
        return None;
    };
    Some((SymbolKind::LnObj, id.to_string().to_ascii_uppercase()))
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
        | BmsChannel::Extension(_)
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
                        Some(DiagnosticCode::DuplicateDefinition),
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
                Some(DiagnosticCode::UndefinedReference),
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
            AnalysisDiagnostic::new(
                error.range(),
                error.severity(),
                error.message(),
                Some(error.code()),
            )
        })
        .collect();
    let mut scratch = SymbolIndex::default();
    collect_lines(
        &source,
        &options.header_prefixes,
        &mut scratch,
        &mut diagnostics,
    );
    check_conditional(&source, &options.header_prefixes, &mut diagnostics);
    add_duplicate_diagnostics(index, &mut diagnostics);
    add_missing_reference_diagnostics(index, &mut diagnostics);
    diagnostics.sort_by_key(|diagnostic| (diagnostic.range().start(), diagnostic.range().end()));
    diagnostics
}
