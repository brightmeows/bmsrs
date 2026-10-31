//! rowan language binding and node aliases.

use rowan::Language;

use crate::SyntaxKind;

/// The rowan language marker for BMS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BmsLanguage;

impl Language for BmsLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from_raw(raw)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.raw()
    }
}

/// BMS CST node.
pub type SyntaxNode = rowan::SyntaxNode<BmsLanguage>;

/// BMS CST token.
pub type SyntaxToken = rowan::SyntaxToken<BmsLanguage>;

/// A node or token.
pub type SyntaxElement = rowan::SyntaxElement<BmsLanguage>;

/// The kind of a node or token.
pub type SyntaxKindRaw = rowan::SyntaxKind;
