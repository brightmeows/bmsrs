//! Resource definition and reference indices.

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

/// A resource definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDefinition {
    /// Resource kind.
    kind: SymbolKind,
    /// Normalized ID (uppercase).
    id: String,
    /// Whole-line range.
    range: TextRange,
    /// Command-name token range.
    name_range: TextRange,
}

impl SymbolDefinition {
    /// Construct a definition.
    pub(crate) const fn new(
        kind: SymbolKind,
        id: String,
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
        &self.id
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
    /// Normalized ID (uppercase).
    id: String,
    /// Reference token range.
    range: TextRange,
}

impl SymbolReference {
    /// Construct a reference.
    pub(crate) const fn new(kind: SymbolKind, id: String, range: TextRange) -> Self {
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
        &self.id
    }

    /// Return the reference range.
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }
}

/// Definition and reference index.
#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    /// Definition list.
    definitions: Vec<SymbolDefinition>,
    /// Reference list.
    references: Vec<SymbolReference>,
}

impl SymbolIndex {
    /// Return all definitions.
    #[must_use]
    pub fn definitions(&self) -> &[SymbolDefinition] {
        &self.definitions
    }

    /// Return all references.
    #[must_use]
    pub fn references(&self) -> &[SymbolReference] {
        &self.references
    }

    /// Look up definitions with the given kind and ID.
    pub fn definitions_for(
        &self,
        kind: SymbolKind,
        id: &str,
    ) -> impl Iterator<Item = &SymbolDefinition> {
        self.definitions.iter().filter(move |definition| {
            definition.kind == kind && definition.id.eq_ignore_ascii_case(id)
        })
    }

    /// Look up references with the given kind and ID.
    pub fn references_for(
        &self,
        kind: SymbolKind,
        id: &str,
    ) -> impl Iterator<Item = &SymbolReference> {
        self.references.iter().filter(move |reference| {
            reference.kind == kind && reference.id.eq_ignore_ascii_case(id)
        })
    }

    /// Return whether a definition exists.
    #[must_use]
    pub fn has_definition(&self, kind: SymbolKind, id: &str) -> bool {
        self.definitions_for(kind, id).next().is_some()
    }

    /// Look up the definition at the given offset.
    #[must_use]
    pub fn definition_at(&self, offset: TextSize) -> Option<&SymbolDefinition> {
        self.definitions
            .iter()
            .find(|definition| contains(definition.name_range, offset))
    }

    /// Look up the reference at the given offset.
    #[must_use]
    pub fn reference_at(&self, offset: TextSize) -> Option<&SymbolReference> {
        self.references
            .iter()
            .find(|reference| contains(reference.range, offset))
    }

    /// Append a definition.
    pub(crate) fn push_definition(&mut self, definition: SymbolDefinition) {
        self.definitions.push(definition);
    }

    /// Append a reference.
    pub(crate) fn push_reference(&mut self, reference: SymbolReference) {
        self.references.push(reference);
    }
}

/// Return whether a range contains an offset.
fn contains(range: TextRange, offset: TextSize) -> bool {
    range.start() <= offset && offset < range.end()
}
