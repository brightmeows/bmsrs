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

/// A resource definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDefinition {
    /// Resource kind.
    kind: SymbolKind,
    /// Normalized ID.
    id: SymbolId,
    /// Whole-line range.
    range: TextRange,
    /// Command-name token range.
    name_range: TextRange,
}

impl SymbolDefinition {
    /// Construct a definition.
    pub(crate) const fn new(
        kind: SymbolKind,
        id: SymbolId,
        range: TextRange,
        name_range: TextRange,
    ) -> Self {
        Self {
            kind,
            id,
            range,
            name_range,
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

    /// Return the whole-line range.
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    /// Return the command-name range.
    #[must_use]
    pub const fn name_range(&self) -> TextRange {
        self.name_range
    }
}

/// A resource reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolReference {
    /// Resource kind.
    kind: SymbolKind,
    /// Normalized ID.
    id: SymbolId,
    /// Reference token range.
    range: TextRange,
}

impl SymbolReference {
    /// Construct a reference.
    pub(crate) const fn new(kind: SymbolKind, id: SymbolId, range: TextRange) -> Self {
        Self { kind, id, range }
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

    /// Return the reference range.
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }
}

/// Definition and reference index.
///
/// Definitions and references are keyed by their start offset, so iteration is
/// source order and `definition_at`/`reference_at` are logarithmic. Secondary
/// maps by `(kind, id)` make `definitions_for`/`references_for` logarithmic as
/// well.
#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    /// Definitions keyed by `name_range.start`.
    definitions: BTreeMap<TextSize, SymbolDefinition>,
    /// References keyed by `range.start`.
    references: BTreeMap<TextSize, SymbolReference>,
    /// Definition starts grouped by `(kind, id)`.
    definition_keys: BTreeMap<(SymbolKind, SymbolId), BTreeSet<TextSize>>,
    /// Reference starts grouped by `(kind, id)`.
    reference_keys: BTreeMap<(SymbolKind, SymbolId), BTreeSet<TextSize>>,
}

impl SymbolIndex {
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
                .filter_map(move |start| self.definitions.get(start))
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
                .filter_map(move |start| self.references.get(start))
        })
    }

    /// Return whether a definition exists.
    #[must_use]
    pub fn has_definition(&self, kind: SymbolKind, id: &str) -> bool {
        SymbolId::new(id).is_some_and(|symbol| self.definition_keys.contains_key(&(kind, symbol)))
    }

    /// Look up the definition at the given offset.
    #[must_use]
    pub fn definition_at(&self, offset: TextSize) -> Option<&SymbolDefinition> {
        self.definitions
            .range(..=offset)
            .next_back()
            .map(|(_, definition)| definition)
            .filter(|definition| contains(definition.name_range, offset))
    }

    /// Look up the reference at the given offset.
    #[must_use]
    pub fn reference_at(&self, offset: TextSize) -> Option<&SymbolReference> {
        self.references
            .range(..=offset)
            .next_back()
            .map(|(_, reference)| reference)
            .filter(|reference| contains(reference.range, offset))
    }

    /// Append a definition.
    pub(crate) fn push_definition(&mut self, definition: SymbolDefinition) {
        let start = definition.name_range.start();
        self.definition_keys
            .entry((definition.kind, definition.id))
            .or_default()
            .insert(start);
        self.definitions.insert(start, definition);
    }

    /// Append a reference.
    pub(crate) fn push_reference(&mut self, reference: SymbolReference) {
        let start = reference.range.start();
        self.reference_keys
            .entry((reference.kind, reference.id))
            .or_default()
            .insert(start);
        self.references.insert(start, reference);
    }

    /// Remove all definitions and references whose start offset falls in
    /// `range`.
    #[expect(dead_code, reason = "used by the incremental analysis update path")]
    pub(crate) fn remove_range(&mut self, range: TextRange) {
        let definition_starts: Vec<_> = self
            .definitions
            .range(range.start()..range.end())
            .map(|(start, _)| *start)
            .collect();
        for start in definition_starts {
            if let Some(definition) = self.definitions.remove(&start) {
                remove_key(
                    &mut self.definition_keys,
                    (definition.kind, definition.id),
                    start,
                );
            }
        }

        let reference_starts: Vec<_> = self
            .references
            .range(range.start()..range.end())
            .map(|(start, _)| *start)
            .collect();
        for start in reference_starts {
            if let Some(reference) = self.references.remove(&start) {
                remove_key(
                    &mut self.reference_keys,
                    (reference.kind, reference.id),
                    start,
                );
            }
        }
    }
}

/// Remove one start from a key map, dropping the key when it becomes empty.
#[expect(dead_code, reason = "used by SymbolIndex::remove_range")]
fn remove_key(
    map: &mut BTreeMap<(SymbolKind, SymbolId), BTreeSet<TextSize>>,
    key: (SymbolKind, SymbolId),
    start: TextSize,
) {
    let empty = map.get_mut(&key).is_some_and(|starts| {
        starts.remove(&start);
        starts.is_empty()
    });
    if empty {
        map.remove(&key);
    }
}

/// Return whether a range contains an offset.
fn contains(range: TextRange, offset: TextSize) -> bool {
    range.start() <= offset && offset < range.end()
}
