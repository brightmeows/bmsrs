//! Typed wrappers over the CST.

use rowan::{TextRange, TextSize};

use crate::{LineKind, Parse, ParseOptions, SyntaxKind, SyntaxNode, SyntaxToken};

/// Document root node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceFile(pub(crate) SyntaxNode);

impl SourceFile {
    /// Parse text and return the CST.
    #[must_use]
    pub fn parse(text: &str) -> Parse {
        Self::parse_with(text, &ParseOptions::default())
    }

    /// Parse text with options.
    #[must_use]
    pub fn parse_with(text: &str, options: &ParseOptions) -> Parse {
        crate::parser::parse_with(text, options)
    }

    /// Parse from a lossless document.
    #[must_use]
    pub fn parse_document(document: &bmsrs_bms_source::Document) -> Parse {
        Self::parse(&document.to_text())
    }

    /// Parse from a lossless document with options.
    #[must_use]
    pub fn parse_document_with(
        document: &bmsrs_bms_source::Document,
        options: &ParseOptions,
    ) -> Parse {
        Self::parse_with(&document.to_text(), options)
    }

    /// Wrap a node as the document root; returns `None` when the kind does not
    /// match.
    #[must_use]
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::SourceFile).then_some(Self(node))
    }

    /// Return the underlying node.
    #[must_use]
    pub const fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    /// Iterate over all lines depth-first.
    pub fn lines(&self) -> impl Iterator<Item = Line> {
        self.0.descendants().filter_map(Line::cast)
    }

    /// Return the token at the given offset.
    #[must_use]
    pub fn token_at_offset(&self, offset: TextSize) -> Option<SyntaxToken> {
        self.0.token_at_offset(offset).right_biased()
    }
}

/// A line (or control-flow directive line).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Line(SyntaxNode);

impl Line {
    /// Wrap a node as a line; returns `None` when the kind does not match.
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

    /// Return the underlying node.
    #[must_use]
    pub const fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    /// Return the node's text range.
    #[must_use]
    pub fn range(&self) -> TextRange {
        self.0.text_range()
    }

    /// Return the line classification.
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

    /// Return whether this is a control-flow directive line.
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

    /// Return the header command name.
    #[must_use]
    pub fn command_name(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::CommandName)
    }

    /// Return the header command value.
    #[must_use]
    pub fn value(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::Value)
    }

    /// Return the measure number.
    #[must_use]
    pub fn measure(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::Measure)
    }

    /// Return the channel number.
    #[must_use]
    pub fn channel(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::Channel)
    }

    /// Iterate over the chunks of the channel body.
    pub fn body_chunks(&self) -> impl Iterator<Item = SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .filter(|token| token.kind() == SyntaxKind::BodyChunk)
    }

    /// Return the content text with leading and trailing trivia removed.
    ///
    /// Inline whitespace between a command and its value is preserved; leading
    /// and trailing whitespace, comments, and the line terminator are removed.
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

    /// Return the direct child token of the given kind.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .find(|token| token.kind() == kind)
    }
}
