//! Resource definition and reference indices.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use bmsrs_bms_syntax::{TextRange, TextSize};

/// Resource kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolKind {
    /// WAV / EXWAV.
    Wav,
    /// BMP / EXBMP.
    Bmp,
    /// BGA / @BGA / SWBGA / ARGB.
    Bga,
    /// BPM / EXBPM.
    Bpm,
    /// STOP.
    Stop,
    /// SCROLL.
    Scroll,
    /// SPEED.
    Speed,
    /// EXRANK.
    ExRank,
    /// CHANGEOPTION.
    ChangeOption,
    /// TEXT / SONG.
    Text,
    /// SEEK.
    Seek,
}

impl SymbolKind {
    /// Return the English label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Wav => "WAV",
            Self::Bmp => "BMP",
            Self::Bga => "BGA",
            Self::Bpm => "BPM",
            Self::Stop => "STOP",
            Self::Scroll => "SCROLL",
            Self::Speed => "SPEED",
            Self::ExRank => "EXRANK",
            Self::ChangeOption => "CHANGEOPTION",
            Self::Text => "TEXT",
            Self::Seek => "SEEK",
        }
    }
}

/// A normalized 1-2 character resource ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId {
    /// ASCII bytes, zero-padded when shorter than two.
    bytes: [u8; 2],
    /// Number of valid bytes.
    len: u8,
}

impl SymbolId {
    /// Construct an ID from ASCII alphanumerics, uppercasing lowercase input.
    #[must_use]
    pub fn new(id: &str) -> Option<Self> {
        let mut bytes = [0u8; 2];
        let mut len = 0u8;
        for byte in id.bytes() {
            if !byte.is_ascii_alphanumeric() || len >= 2 {
                return None;
            }
            let slot = bytes.get_mut(usize::from(len))?;
            *slot = byte.to_ascii_uppercase();
            len = len.saturating_add(1);
        }
        (len != 0).then_some(Self { bytes, len })
    }

    /// Return the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.bytes
            .get(..usize::from(self.len))
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or_default()
    }
}

impl PartialOrd for SymbolId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SymbolId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// A zero-based line and byte offset within that line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SourcePos {
    /// Zero-based line number.
    line: u32,
    /// Byte offset within the line.
    offset: u32,
}

impl SourcePos {
    /// Construct a source position.
    const fn new(line: u32, offset: u32) -> Self {
        Self { line, offset }
    }
}

/// A resource definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDefinition {
    /// Resource kind.
    kind: SymbolKind,
    /// Normalized ID.
    id: SymbolId,
    /// Zero-based line number.
    line: u32,
    /// Whole-line start offset.
    start: u32,
    /// Whole-line end offset.
    end: u32,
    /// Command-name start offset.
    name_start: u32,
    /// Command-name end offset.
    name_end: u32,
}

impl SymbolDefinition {
    /// Construct a definition.
    pub(crate) const fn new(
        kind: SymbolKind,
        id: SymbolId,
        line: u32,
        start: u32,
        end: u32,
        name_start: u32,
        name_end: u32,
    ) -> Self {
        Self {
            kind,
            id,
            line,
            start,
            end,
            name_start,
            name_end,
        }
    }

    /// Return the resource kind.
    #[must_use]
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }

    /// Return the ID.
    #[must_use]
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Return the zero-based line number.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// Return the whole-line start offset within the line.
    #[must_use]
    pub const fn start(&self) -> u32 {
        self.start
    }

    /// Return the whole-line end offset within the line.
    #[must_use]
    pub const fn end(&self) -> u32 {
        self.end
    }

    /// Return the command-name start offset within the line.
    #[must_use]
    pub const fn name_start(&self) -> u32 {
        self.name_start
    }

    /// Return the command-name end offset within the line.
    #[must_use]
    pub const fn name_end(&self) -> u32 {
        self.name_end
    }
}

/// A resource reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolReference {
    /// Resource kind.
    kind: SymbolKind,
    /// Normalized ID.
    id: SymbolId,
    /// Zero-based line number.
    line: u32,
    /// Reference start offset within the line.
    start: u32,
    /// Reference end offset within the line.
    end: u32,
}

impl SymbolReference {
    /// Construct a reference.
    pub(crate) const fn new(
        kind: SymbolKind,
        id: SymbolId,
        line: u32,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            kind,
            id,
            line,
            start,
            end,
        }
    }

    /// Return the resource kind.
    #[must_use]
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }

    /// Return the ID.
    #[must_use]
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Return the zero-based line number.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// Return the reference start offset within the line.
    #[must_use]
    pub const fn start(&self) -> u32 {
        self.start
    }

    /// Return the reference end offset within the line.
    #[must_use]
    pub const fn end(&self) -> u32 {
        self.end
    }
}

/// Definition and reference index.
///
/// Entries are keyed by `(line, offset)` so an edit inside one line only
/// touches that line's entries. The absolute line starts are kept separately
/// and refreshed after each parse; absolute `TextRange` values are computed on
/// demand.
#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    /// Start offset of each line.
    line_starts: Vec<TextSize>,
    /// Definitions keyed by `(line, name_start)`.
    definitions: BTreeMap<SourcePos, SymbolDefinition>,
    /// References keyed by `(line, start)`.
    references: BTreeMap<SourcePos, SymbolReference>,
    /// Definition keys grouped by `(kind, id)`.
    definition_keys: BTreeMap<(SymbolKind, SymbolId), BTreeSet<SourcePos>>,
    /// Reference keys grouped by `(kind, id)`.
    reference_keys: BTreeMap<(SymbolKind, SymbolId), BTreeSet<SourcePos>>,
}

impl SymbolIndex {
    /// Replace the line-start table.
    pub(crate) fn set_line_starts(&mut self, starts: Vec<TextSize>) {
        self.line_starts = starts;
    }

    /// Iterate over all definitions in source order.
    pub fn definitions(&self) -> impl Iterator<Item = &SymbolDefinition> {
        self.definitions.values()
    }

    /// Iterate over all references in source order.
    pub fn references(&self) -> impl Iterator<Item = &SymbolReference> {
        self.references.values()
    }

    /// Look up definitions with the given kind and ID.
    pub fn definitions_for(
        &self,
        kind: SymbolKind,
        id: &str,
    ) -> impl Iterator<Item = &SymbolDefinition> {
        let symbol_key = SymbolId::new(id).map(|symbol| (kind, symbol));
        let starts = symbol_key.and_then(|key| self.definition_keys.get(&key));
        starts.into_iter().flat_map(move |start_set| {
            start_set
                .iter()
                .filter_map(move |pos| self.definitions.get(pos))
        })
    }

    /// Look up references with the given kind and ID.
    pub fn references_for(
        &self,
        kind: SymbolKind,
        id: &str,
    ) -> impl Iterator<Item = &SymbolReference> {
        let symbol_key = SymbolId::new(id).map(|symbol| (kind, symbol));
        let starts = symbol_key.and_then(|key| self.reference_keys.get(&key));
        starts.into_iter().flat_map(move |start_set| {
            start_set
                .iter()
                .filter_map(move |pos| self.references.get(pos))
        })
    }

    /// Return whether a definition exists.
    #[must_use]
    pub fn has_definition(&self, kind: SymbolKind, id: &str) -> bool {
        SymbolId::new(id).is_some_and(|symbol| self.definition_keys.contains_key(&(kind, symbol)))
    }

    /// Look up the definition at an absolute byte offset.
    #[must_use]
    pub fn definition_at(&self, offset: TextSize) -> Option<&SymbolDefinition> {
        let (line, line_offset) = self.position(offset)?;
        let pos = SourcePos::new(line, line_offset);
        self.definitions
            .range(..=pos)
            .next_back()
            .map(|(_, definition)| definition)
            .filter(|definition| {
                definition.line == line
                    && definition.name_start <= line_offset
                    && line_offset < definition.name_end
            })
    }

    /// Look up the reference at an absolute byte offset.
    #[must_use]
    pub fn reference_at(&self, offset: TextSize) -> Option<&SymbolReference> {
        let (line, line_offset) = self.position(offset)?;
        let pos = SourcePos::new(line, line_offset);
        self.references
            .range(..=pos)
            .next_back()
            .map(|(_, reference)| reference)
            .filter(|reference| {
                reference.line == line
                    && reference.start <= line_offset
                    && line_offset < reference.end
            })
    }

    /// Return the absolute range of a definition.
    #[must_use]
    pub fn range_of_definition(&self, definition: &SymbolDefinition) -> Option<TextRange> {
        self.range_on_line(definition.line, definition.start, definition.end)
    }

    /// Return the absolute command-name range of a definition.
    #[must_use]
    pub fn name_range_of_definition(&self, definition: &SymbolDefinition) -> Option<TextRange> {
        self.range_on_line(definition.line, definition.name_start, definition.name_end)
    }

    /// Return the absolute range of a reference.
    #[must_use]
    pub fn range_of_reference(&self, reference: &SymbolReference) -> Option<TextRange> {
        self.range_on_line(reference.line, reference.start, reference.end)
    }

    /// Append a definition.
    pub(crate) fn push_definition(&mut self, definition: SymbolDefinition) {
        let pos = SourcePos::new(definition.line, definition.name_start);
        self.definition_keys
            .entry((definition.kind, definition.id))
            .or_default()
            .insert(pos);
        self.definitions.insert(pos, definition);
    }

    /// Append a reference.
    pub(crate) fn push_reference(&mut self, reference: SymbolReference) {
        let pos = SourcePos::new(reference.line, reference.start);
        self.reference_keys
            .entry((reference.kind, reference.id))
            .or_default()
            .insert(pos);
        self.references.insert(pos, reference);
    }

    /// Remove all definitions and references on one line.
    pub(crate) fn remove_line(&mut self, line: u32) {
        let start = SourcePos::new(line, 0);
        let end = SourcePos::new(line.saturating_add(1), 0);

        let definition_keys: Vec<_> = self
            .definitions
            .range(start..end)
            .map(|(pos, _)| *pos)
            .collect();
        for pos in definition_keys {
            if let Some(definition) = self.definitions.remove(&pos) {
                remove_key(
                    &mut self.definition_keys,
                    (definition.kind, definition.id),
                    pos,
                );
            }
        }

        let reference_keys: Vec<_> = self
            .references
            .range(start..end)
            .map(|(pos, _)| *pos)
            .collect();
        for pos in reference_keys {
            if let Some(reference) = self.references.remove(&pos) {
                remove_key(
                    &mut self.reference_keys,
                    (reference.kind, reference.id),
                    pos,
                );
            }
        }
    }

    /// Convert an absolute offset into `(line, line_offset)`.
    fn position(&self, offset: TextSize) -> Option<(u32, u32)> {
        let line = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .checked_sub(1)?;
        let line_start = *self.line_starts.get(line)?;
        Some((u32::try_from(line).ok()?, u32::from(offset - line_start)))
    }

    /// Convert a line-relative range into an absolute range.
    fn range_on_line(&self, line: u32, start: u32, end: u32) -> Option<TextRange> {
        let line_start = *self.line_starts.get(usize::try_from(line).ok()?)?;
        Some(TextRange::new(
            line_start + TextSize::from(start),
            line_start + TextSize::from(end),
        ))
    }
}

/// Remove one position from a key map, dropping the key when it becomes empty.
fn remove_key(
    map: &mut BTreeMap<(SymbolKind, SymbolId), BTreeSet<SourcePos>>,
    key: (SymbolKind, SymbolId),
    pos: SourcePos,
) {
    let empty = map.get_mut(&key).is_some_and(|positions| {
        positions.remove(&pos);
        positions.is_empty()
    });
    if empty {
        map.remove(&key);
    }
}
