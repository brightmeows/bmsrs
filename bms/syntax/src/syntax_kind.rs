//! CST 的 token 与节点种类。

/// BMS CST 的 token 与节点种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    /// 空白。
    Whitespace,
    /// 行终止符。
    Newline,
    /// 注释。
    Comment,
    /// `#` 前缀。
    Hash,
    /// `%` 前缀。
    Percent,
    /// 头部命令名。
    CommandName,
    /// 头部命令值。
    Value,
    /// 小节号。
    Measure,
    /// 通道号。
    Channel,
    /// 通道行冒号。
    Colon,
    /// 通道体中的两字符值。
    BodyChunk,
    /// 普通文本。
    Text,
    /// 无法归类的字符。
    ErrorToken,

    /// 文档根节点。
    SourceFile,
    /// 头部命令行。
    HeaderLine,
    /// 通道数据行。
    MessageLine,
    /// 注释行。
    CommentLine,
    /// 空白行。
    BlankLine,
    /// 普通文本行。
    TextLine,
    /// 解析失败的行。
    ErrorLine,
    /// 头部命令节点。
    HeaderCommand,
    /// 头部值节点。
    HeaderValue,
    /// 通道行头节点。
    MessageHeader,
    /// 通道体节点。
    MessageBody,

    /// `#RANDOM` 块。
    RandomBlock,
    /// `#IF` 互斥链。
    RandomChain,
    /// 互斥链中的单个分支。
    RandomBranch,
    /// `#RANDOM` / `#SETRANDOM` 指令。
    RandomDirective,
    /// `#IF` 指令。
    IfDirective,
    /// `#ELSEIF` 指令。
    ElseIfDirective,
    /// `#ELSE` 指令。
    ElseDirective,
    /// `#ENDIF` 指令。
    EndIfDirective,
    /// `#ENDRANDOM` 指令。
    EndRandomDirective,

    /// `#SWITCH` 块。
    SwitchBlock,
    /// `#SWITCH` 的单个 case。
    SwitchCase,
    /// `#SWITCH` / `#SETSWITCH` 指令。
    SwitchDirective,
    /// `#CASE` 指令。
    CaseDirective,
    /// `#SKIP` 指令。
    SkipDirective,
    /// `#DEF` 指令。
    DefDirective,
    /// `#ENDSW` 指令。
    EndSwDirective,

    /// 通用错误节点。
    ErrorNode,
    /// 自定义头部前缀（非 `#` / `%`）。
    Prefix,
    /// 未知种类，仅用于 `kind_from_raw` 的兜底。
    Unknown,
}

impl SyntaxKind {
    /// 转换为 rowan 的原始种类。
    pub(crate) const fn raw(self) -> rowan::SyntaxKind {
        rowan::SyntaxKind(self as u16)
    }

    /// 从 rowan 原始种类还原。
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

    /// 是否是 trivia。
    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment | Self::Newline)
    }

    /// 是否是控制流指令节点。
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
