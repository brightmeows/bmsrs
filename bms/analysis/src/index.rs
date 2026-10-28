//! 资源定义与引用索引。

use bms_syntax::{TextRange, TextSize};

/// 资源种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolKind {
    /// WAV / EXWAV。
    Wav,
    /// BMP / EXBMP。
    Bmp,
    /// BGA / @BGA / SWBGA / ARGB。
    Bga,
    /// BPM / EXBPM。
    Bpm,
    /// STOP。
    Stop,
    /// SCROLL。
    Scroll,
    /// SPEED。
    Speed,
    /// EXRANK。
    ExRank,
    /// CHANGEOPTION。
    ChangeOption,
    /// TEXT / SONG。
    Text,
    /// SEEK。
    Seek,
}

impl SymbolKind {
    /// 返回中文标签。
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

/// 一个资源定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDefinition {
    /// 资源种类。
    kind: SymbolKind,
    /// 规范化 ID（大写）。
    id: String,
    /// 整行范围。
    range: TextRange,
    /// 命令名 token 范围。
    name_range: TextRange,
}

impl SymbolDefinition {
    /// 构造定义。
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

    /// 返回资源种类。
    #[must_use]
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }

    /// 返回 ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回整行范围。
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    /// 返回命令名范围。
    #[must_use]
    pub const fn name_range(&self) -> TextRange {
        self.name_range
    }
}

/// 一个资源引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolReference {
    /// 资源种类。
    kind: SymbolKind,
    /// 规范化 ID（大写）。
    id: String,
    /// 引用 token 范围。
    range: TextRange,
}

impl SymbolReference {
    /// 构造引用。
    pub(crate) const fn new(kind: SymbolKind, id: String, range: TextRange) -> Self {
        Self { kind, id, range }
    }

    /// 返回资源种类。
    #[must_use]
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }

    /// 返回 ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 返回引用范围。
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }
}

/// 定义与引用索引。
#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    /// 定义列表。
    definitions: Vec<SymbolDefinition>,
    /// 引用列表。
    references: Vec<SymbolReference>,
}

impl SymbolIndex {
    /// 返回所有定义。
    #[must_use]
    pub fn definitions(&self) -> &[SymbolDefinition] {
        &self.definitions
    }

    /// 返回所有引用。
    #[must_use]
    pub fn references(&self) -> &[SymbolReference] {
        &self.references
    }

    /// 查找指定种类与 ID 的定义。
    pub fn definitions_for(
        &self,
        kind: SymbolKind,
        id: &str,
    ) -> impl Iterator<Item = &SymbolDefinition> {
        self.definitions.iter().filter(move |definition| {
            definition.kind == kind && definition.id.eq_ignore_ascii_case(id)
        })
    }

    /// 查找指定种类与 ID 的引用。
    pub fn references_for(
        &self,
        kind: SymbolKind,
        id: &str,
    ) -> impl Iterator<Item = &SymbolReference> {
        self.references.iter().filter(move |reference| {
            reference.kind == kind && reference.id.eq_ignore_ascii_case(id)
        })
    }

    /// 判断是否存在定义。
    #[must_use]
    pub fn has_definition(&self, kind: SymbolKind, id: &str) -> bool {
        self.definitions_for(kind, id).next().is_some()
    }

    /// 查找指定偏移处的定义。
    #[must_use]
    pub fn definition_at(&self, offset: TextSize) -> Option<&SymbolDefinition> {
        self.definitions
            .iter()
            .find(|definition| contains(definition.name_range, offset))
    }

    /// 查找指定偏移处的引用。
    #[must_use]
    pub fn reference_at(&self, offset: TextSize) -> Option<&SymbolReference> {
        self.references
            .iter()
            .find(|reference| contains(reference.range, offset))
    }

    /// 追加定义。
    pub(crate) fn push_definition(&mut self, definition: SymbolDefinition) {
        self.definitions.push(definition);
    }

    /// 追加引用。
    pub(crate) fn push_reference(&mut self, reference: SymbolReference) {
        self.references.push(reference);
    }
}

/// 判断范围是否包含偏移。
fn contains(range: TextRange, offset: TextSize) -> bool {
    range.start() <= offset && offset < range.end()
}
