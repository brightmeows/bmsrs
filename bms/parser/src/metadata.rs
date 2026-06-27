//! 乐曲级元数据字段。
//!
//! 对应分词器的 [`BmsHeaderMetadata`]。

use std::collections::BTreeMap;

use bms_tokenizer::{BmsHeaderMetadata, TextIndex};

/// 乐曲 / 谱面识别元数据。
///
/// 所有字段均使用最后胜出语义。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// 乐曲标题（`#TITLE`）。
    pub title: Option<String>,
    /// 乐曲副标题（`#SUBTITLE`）。
    pub subtitle: Option<String>,
    /// 主要艺术家 / 作曲者（`#ARTIST`）。
    pub artist: Option<String>,
    /// 合作创作者（`#SUBARTIST`）。
    pub sub_artist: Option<String>,
    /// 音乐流派（`#GENRE` / `#GENLE`）。
    pub genre: Option<String>,
    /// BMS 谱面作者名（`#MAKER`）。
    pub maker: Option<String>,
    /// 选曲列表中显示的文本（`#COMMENT`）。
    pub comment: Option<String>,
    /// 字符编码提示（`#CHARSET`）。
    pub charset: Option<String>,
    /// 作者网站 URL（`%URL`）。
    pub url: Option<String>,
    /// 作者邮箱地址（`%EMAIL`）。
    pub email: Option<String>,
    /// 定时屏幕文本定义（`#TEXTxx`、`#SONGxx`）。
    pub text_defs: BTreeMap<TextIndex, String>,
}

impl Metadata {
    /// 将一个元数据头部命令应用到此结构体。
    pub fn apply<C: AsRef<str>>(&mut self, header: &BmsHeaderMetadata<C>) {
        match header {
            BmsHeaderMetadata::Title(s) => self.title = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Subtitle(s) => self.subtitle = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Artist(s) => self.artist = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::SubArtist(s) => self.sub_artist = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Genre(s) => self.genre = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Maker(s) => self.maker = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Comment(s) => self.comment = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Charset(s) => self.charset = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Url(s) => self.url = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Email(s) => self.email = Some(s.as_ref().to_owned()),
            BmsHeaderMetadata::Text { id, value } => {
                self.text_defs.insert(*id, value.as_ref().to_owned());
            }
        }
    }
}
