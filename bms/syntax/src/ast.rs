//! CST 的类型化包装。

use rowan::{TextRange, TextSize};

use crate::{LineKind, Parse, ParseOptions, SyntaxKind, SyntaxNode, SyntaxToken};

/// 文档根节点。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceFile(pub(crate) SyntaxNode);

impl SourceFile {
    /// 解析文本并返回 CST。
    #[must_use]
    pub fn parse(text: &str) -> Parse {
        Self::parse_with(text, &ParseOptions::default())
    }

    /// 使用选项解析文本。
    #[must_use]
    pub fn parse_with(text: &str, options: &ParseOptions) -> Parse {
        crate::parser::parse_with(text, options)
    }

    /// 从无损文档解析。
    #[must_use]
    pub fn parse_document(document: &bmsrs_bms_source::Document) -> Parse {
        Self::parse(&document.to_text())
    }

    /// 从无损文档按选项解析。
    #[must_use]
    pub fn parse_document_with(
        document: &bmsrs_bms_source::Document,
        options: &ParseOptions,
    ) -> Parse {
        Self::parse_with(&document.to_text(), options)
    }

    /// 把节点包装为文档根；种类不匹配时返回 `None`。
    #[must_use]
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::SourceFile).then_some(Self(node))
    }

    /// 返回底层节点。
    #[must_use]
    pub const fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    /// 深度优先迭代所有行。
    pub fn lines(&self) -> impl Iterator<Item = Line> {
        self.0.descendants().filter_map(Line::cast)
    }

    /// 返回指定偏移处的 token。
    #[must_use]
    pub fn token_at_offset(&self, offset: TextSize) -> Option<SyntaxToken> {
        self.0.token_at_offset(offset).right_biased()
    }
}

/// 一行（或控制流指令行）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Line(SyntaxNode);

impl Line {
    /// 把节点包装为行；种类不匹配时返回 `None`。
    #[must_use]
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        let is_line = matches!(
            node.kind(),
            SyntaxKind::HeaderLine
                | SyntaxKind::MessageLine
                | SyntaxKind::CommentLine
                | SyntaxKind::BlankLine
                | SyntaxKind::TextLine
                | SyntaxKind::ErrorLine
                | SyntaxKind::RandomDirective
                | SyntaxKind::IfDirective
                | SyntaxKind::ElseIfDirective
                | SyntaxKind::ElseDirective
                | SyntaxKind::EndIfDirective
                | SyntaxKind::EndRandomDirective
                | SyntaxKind::SwitchDirective
                | SyntaxKind::CaseDirective
                | SyntaxKind::SkipDirective
                | SyntaxKind::DefDirective
                | SyntaxKind::EndSwDirective
        );
        is_line.then_some(Self(node))
    }

    /// 返回底层节点。
    #[must_use]
    pub const fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    /// 返回节点文本范围。
    #[must_use]
    pub fn range(&self) -> TextRange {
        self.0.text_range()
    }

    /// 返回行分类。
    #[must_use]
    pub fn kind(&self) -> LineKind {
        match self.0.kind() {
            SyntaxKind::BlankLine => LineKind::Blank,
            SyntaxKind::CommentLine => LineKind::Comment,
            SyntaxKind::MessageLine => LineKind::Message,
            SyntaxKind::TextLine => LineKind::Text,
            SyntaxKind::ErrorLine => LineKind::Error,
            _ => LineKind::Header,
        }
    }

    /// 是否是控制流指令行。
    #[must_use]
    pub fn is_directive(&self) -> bool {
        matches!(
            self.0.kind(),
            SyntaxKind::RandomDirective
                | SyntaxKind::IfDirective
                | SyntaxKind::ElseIfDirective
                | SyntaxKind::ElseDirective
                | SyntaxKind::EndIfDirective
                | SyntaxKind::EndRandomDirective
                | SyntaxKind::SwitchDirective
                | SyntaxKind::CaseDirective
                | SyntaxKind::SkipDirective
                | SyntaxKind::DefDirective
                | SyntaxKind::EndSwDirective
        )
    }

    /// 返回头部命令名。
    #[must_use]
    pub fn command_name(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::CommandName)
    }

    /// 返回头部命令值。
    #[must_use]
    pub fn value(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::Value)
    }

    /// 返回小节号。
    #[must_use]
    pub fn measure(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::Measure)
    }

    /// 返回通道号。
    #[must_use]
    pub fn channel(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::Channel)
    }

    /// 迭代通道体的 chunk。
    pub fn body_chunks(&self) -> impl Iterator<Item = SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .filter(|token| token.kind() == SyntaxKind::BodyChunk)
    }

    /// 返回去掉前后 trivia 后的内容文本。
    ///
    /// 行内空白（命令与值之间）会保留，前后空白、注释与行终止符被去掉。
    #[must_use]
    pub fn content_text(&self) -> Option<String> {
        let tokens: Vec<_> = self
            .0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .collect();
        let start = tokens.iter().position(|token| !token.kind().is_trivia())?;
        let end = tokens.iter().rposition(|token| !token.kind().is_trivia())?;
        Some(
            tokens
                .get(start..=end)?
                .iter()
                .map(SyntaxToken::text)
                .collect(),
        )
    }

    /// 返回指定种类的直接子 token。
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .find(|token| token.kind() == kind)
    }
}
