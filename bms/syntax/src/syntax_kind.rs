//! Token and node kinds for the CST.

/// Token and node kinds for the BMS CST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    /// Whitespace.
    Whitespace,
    /// Line terminator.
    Newline,
    /// Comment.
    Comment,
    /// `#` prefix.
    Hash,
    /// `%` prefix.
    Percent,
    /// Header command name.
    CommandName,
    /// Header command value.
    Value,
    /// Measure number.
    Measure,
    /// Channel number.
    Channel,
    /// Channel line colon.
    Colon,
    /// Two-character value in a channel body.
    BodyChunk,
    /// Plain text.
    Text,
    /// A character that could not be classified.
    ErrorToken,

    /// Document root node.
    SourceFile,
    /// Header command line.
    HeaderLine,
    /// Channel data line.
    MessageLine,
    /// Comment line.
    CommentLine,
    /// Blank line.
    BlankLine,
    /// Plain text line.
    TextLine,
    /// A line that failed to parse.
    ErrorLine,
    /// Header command node.
    HeaderCommand,
    /// Header value node.
    HeaderValue,
    /// Channel line header node.
    MessageHeader,
    /// Channel body node.
    MessageBody,

    /// `#RANDOM` block.
    RandomBlock,
    /// `#IF` mutually exclusive chain.
    RandomChain,
    /// A single branch in a mutually exclusive chain.
    RandomBranch,
    /// `#RANDOM` / `#SETRANDOM` directive.
    RandomDirective,
    /// `#IF` directive.
    IfDirective,
    /// `#ELSEIF` directive.
    ElseIfDirective,
    /// `#ELSE` directive.
    ElseDirective,
    /// `#ENDIF` directive.
    EndIfDirective,
    /// `#ENDRANDOM` directive.
    EndRandomDirective,

    /// `#SWITCH` block.
    SwitchBlock,
    /// A single `#SWITCH` case.
    SwitchCase,
    /// `#SWITCH` / `#SETSWITCH` directive.
    SwitchDirective,
    /// `#CASE` directive.
    CaseDirective,
    /// `#SKIP` directive.
    SkipDirective,
    /// `#DEF` directive.
    DefDirective,
    /// `#ENDSW` directive.
    EndSwDirective,

    /// Generic error node.
    ErrorNode,
    /// Custom header prefix (neither `#` nor `%`).
    Prefix,
    /// Unknown kind; only used as the fallback in `kind_from_raw`.
    Unknown,
}

impl SyntaxKind {
    /// Convert to a rowan raw kind.
    pub(crate) const fn raw(self) -> rowan::SyntaxKind {
        rowan::SyntaxKind(self as u16)
    }

    /// Reconstruct from a rowan raw kind.
    pub(crate) const fn from_raw(raw: rowan::SyntaxKind) -> Self {
        match raw.0 {
            0 => Self::Whitespace,
            1 => Self::Newline,
            2 => Self::Comment,
            3 => Self::Hash,
            4 => Self::Percent,
            5 => Self::CommandName,
            6 => Self::Value,
            7 => Self::Measure,
            8 => Self::Channel,
            9 => Self::Colon,
            10 => Self::BodyChunk,
            11 => Self::Text,
            12 => Self::ErrorToken,
            13 => Self::SourceFile,
            14 => Self::HeaderLine,
            15 => Self::MessageLine,
            16 => Self::CommentLine,
            17 => Self::BlankLine,
            18 => Self::TextLine,
            19 => Self::ErrorLine,
            20 => Self::HeaderCommand,
            21 => Self::HeaderValue,
            22 => Self::MessageHeader,
            23 => Self::MessageBody,
            24 => Self::RandomBlock,
            25 => Self::RandomChain,
            26 => Self::RandomBranch,
            27 => Self::RandomDirective,
            28 => Self::IfDirective,
            29 => Self::ElseIfDirective,
            30 => Self::ElseDirective,
            31 => Self::EndIfDirective,
            32 => Self::EndRandomDirective,
            33 => Self::SwitchBlock,
            34 => Self::SwitchCase,
            35 => Self::SwitchDirective,
            36 => Self::CaseDirective,
            37 => Self::SkipDirective,
            38 => Self::DefDirective,
            39 => Self::EndSwDirective,
            40 => Self::ErrorNode,
            41 => Self::Prefix,
            _ => Self::Unknown,
        }
    }

    /// Return whether this kind is trivia.
    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment | Self::Newline)
    }

    /// Return whether this is a control-flow directive node.
    #[must_use]
    pub const fn is_directive(self) -> bool {
        matches!(
            self,
            Self::RandomDirective
                | Self::IfDirective
                | Self::ElseIfDirective
                | Self::ElseDirective
                | Self::EndIfDirective
                | Self::EndRandomDirective
                | Self::SwitchDirective
                | Self::CaseDirective
                | Self::SkipDirective
                | Self::DefDirective
                | Self::EndSwDirective
        )
    }
}
