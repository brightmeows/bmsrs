//! 乐曲/谱面元数据头部：`#TITLE`、`#SUBTITLE`、`#ARTIST`、
//! `#SUBARTIST`、`#GENRE`、`#MAKER`、`#COMMENT`、`#TEXT`/`#SONG`、
//! `#CHARSET`、`%URL`、`%EMAIL`。

use crate::BmsTokenAttr;
use crate::index::TextIndex;
use crate::{BmsHeader, BmsTryFromError};

/// 乐曲/谱面元数据头部。
///
/// 这些命令标识谱面及其作者。它们不携带任何
/// 游玩效果——纯粹是信息性的。
#[derive(Debug, Clone, PartialEq, Eq, BmsTokenAttr)]
pub enum BmsHeaderMetadata<C> {
    /// `#TITLE`——乐曲标题。
    ///
    /// **不应省略**——某些播放器在缺失时会崩溃
    /// （nanasi）。规范中无长度限制，但某些播放器会截断
    /// 或在过长标题时崩溃（DDR：500 字节限制）。可能根据
    /// 文件编码含有多字节字符。
    #[bms_token("#TITLE {}")]
    Title(C),
    /// `#SUBTITLE`——显式副标题（nanasi 扩展）。
    ///
    /// 优于传统的“隐式副标题”解析——后者使用 `#TITLE` 内的
    /// 分隔符（`-`、`~`、`()`、`[]`、`<>`）来拆分标题。
    /// 隐式副标题处理因播放器而异。
    ///
    /// Sonorous 支持多行 `#SUBTITLE`。
    #[bms_token("#SUBTITLE {}")]
    Subtitle(C),
    /// `#ARTIST`——乐曲艺术家 / 作曲者。
    #[bms_token("#ARTIST {}")]
    Artist(C),
    /// `#SUBARTIST`——联合作者（LR2 扩展）。
    ///
    /// 通常用于 BGA 作者、谱师等。在支持的播放器中与
    /// `#ARTIST` 显示方式不同。
    /// `TechnicalGroove` 支持多行 `#SUBARTIST`。
    #[bms_token("#SUBARTIST {}")]
    SubArtist(C),
    /// `#GENRE` 或 `#GENLE`——音乐流派。
    ///
    /// `#GENLE` 是拼写错误别名（uBMplay）；两者映射到同一变体。
    /// 省略时默认：空字符串。
    #[bms_token("#GENRE {}")]
    #[bms_token("#GENLE {}")]
    Genre(C),
    /// `#MAKER`——BMS 谱面作者名（bemaniaDX 扩展）。
    ///
    /// 区分谱师与音乐作曲者。游玩时不显示——
    /// 纯元数据。
    #[bms_token("#MAKER {}")]
    Maker(C),
    /// `#COMMENT`——在选曲列表中显示的文本（pomu 扩展）。
    ///
    /// 可以用双引号包裹以表示空字符串，但解析器
    /// 不应依赖引号存在（旧谱面会省略它们）。
    /// Sonorous 支持多行 `#COMMENT`。
    #[bms_token("#COMMENT {}")]
    Comment(C),
    /// `#TEXT[00-ZZ]` 或 `#SONG[01-ZZ]`——定时屏幕文字（pomu 扩展）。
    ///
    /// 被通道 `#xxx99` 引用。nanasi 在 miss 时显示 `#TEXT00`。
    /// `#SONG` 是过时的别名——优先使用 `#TEXT`。
    ///
    /// 值可以选择性地用双引号包裹，但解析器
    /// 不应依赖引号存在。
    #[bms_token("#TEXT{id} {value}")]
    #[bms_token("#SONG{id} {value}")]
    Text {
        /// 2 字符索引（例如 `"00"`、`"aa"`）。
        id: TextIndex,
        /// 文本内容。
        value: C,
    },
    /// `#CHARSET`——字符编码提示（ruvit 扩展，现已废弃）。
    ///
    /// 取值：`EUC-KR`、`SHIFT-JIS`、`UTF-8`。现代 ruvit
    /// （2.0b5p2+）会自动检测编码并忽略此命令。对于新谱面，
    /// 请保存为 UTF-8（带或不带 BOM）。
    #[bms_token("#CHARSET {}")]
    Charset(C),
    /// `%URL`——作者网站 URL（BMS Manager 扩展）。
    ///
    /// **注意**：BMSE 与 iBMSC 在保存时会删除 `%URL`。
    #[bms_token("%URL {}")]
    Url(C),
    /// `%EMAIL`——作者邮箱地址（BMS Manager 扩展）。
    ///
    /// **注意**：BMSE 与 iBMSC 在保存时会删除 `%EMAIL`。
    #[bms_token("%EMAIL {}")]
    Email(C),
}

// From / TryFrom 转换

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderMetadata<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Metadata(m) => Ok(m),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}
