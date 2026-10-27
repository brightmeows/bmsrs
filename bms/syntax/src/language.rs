//! rowan 语言绑定与节点别名。

use rowan::Language;

use crate::SyntaxKind;

/// BMS 的 rowan 语言标记。
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

/// BMS CST 节点。
pub type SyntaxNode = rowan::SyntaxNode<BmsLanguage>;

/// BMS CST token。
pub type SyntaxToken = rowan::SyntaxToken<BmsLanguage>;

/// 节点或 token。
pub type SyntaxElement = rowan::SyntaxElement<BmsLanguage>;

/// 节点或 token 的种类。
pub type SyntaxKindRaw = rowan::SyntaxKind;
