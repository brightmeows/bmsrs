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
    pub fn apply(&mut self, header: &BmsHeaderMetadata) {
        match header {
            BmsHeaderMetadata::Title(s) => self.title = Some(s.clone()),
            BmsHeaderMetadata::Subtitle(s) => self.subtitle = Some(s.clone()),
            BmsHeaderMetadata::Artist(s) => self.artist = Some(s.clone()),
            BmsHeaderMetadata::SubArtist(s) => self.sub_artist = Some(s.clone()),
            BmsHeaderMetadata::Genre(s) => self.genre = Some(s.clone()),
            BmsHeaderMetadata::Maker(s) => self.maker = Some(s.clone()),
            BmsHeaderMetadata::Comment(s) => self.comment = Some(s.clone()),
            BmsHeaderMetadata::Charset(s) => self.charset = Some(s.clone()),
            BmsHeaderMetadata::Url(s) => self.url = Some(s.clone()),
            BmsHeaderMetadata::Email(s) => self.email = Some(s.clone()),
            BmsHeaderMetadata::Text { id, value } => {
                self.text_defs.insert(*id, value.clone());
            }
        }
    }

    /// 从标题中解析隐式副标题。
    ///
    /// BMS 规范允许 `#TITLE` 值使用分隔符暗示副标题。优先级（首个匹配
    /// 胜出）：`-` → `～` → `()` → `[]` → `<>`。
    ///
    /// 如果已通过 `#SUBTITLE` 显式设置了副标题，则不执行任何操作。
    pub fn parse_implicit_subtitle(&mut self) {
        if self.subtitle.is_some() {
            return;
        }
        let title = match &self.title {
            Some(t) if !t.is_empty() => t.clone(),
            _ => return,
        };

        let result = parse_mirrored(&title, '-')
            .or_else(|| parse_mirrored(&title, '～'))
            .or_else(|| parse_enclosed(&title, '(', ')'))
            .or_else(|| parse_enclosed(&title, '[', ']'))
            .or_else(|| parse_enclosed(&title, '<', '>'));

        if let Some((main, sub)) = result {
            self.title = Some(main.to_owned());
            self.subtitle = Some(sub.to_owned());
        }
    }
}

/// 解析 `sep...sep` 结尾模式（如 `-sub-` 或 `～sub～`）。
fn parse_mirrored(input: &str, sep: char) -> Option<(&str, &str)> {
    let trimmed = input.trim_end();
    let body = trimmed.strip_suffix(sep)?;
    let sep_pos = body.rfind(sep)?;
    if sep_pos == 0 {
        return None;
    }
    let (head, tail) = body.split_at(sep_pos);
    let raw_sub = tail.get(sep.len_utf8()..)?;
    let main = head.trim_end();
    let sub = raw_sub.trim();
    if sub.is_empty() {
        return None;
    }
    Some((main, sub))
}

/// 解析 `open...close` 结尾模式（如 `(sub)`、`[sub]`、`<sub>`）。
fn parse_enclosed(input: &str, open: char, close: char) -> Option<(&str, &str)> {
    let trimmed = input.trim_end();
    let body = trimmed.strip_suffix(close)?;
    let open_pos = body.rfind(open)?;
    if open_pos == 0 {
        return None;
    }
    let (head, tail) = body.split_at(open_pos);
    let raw_sub = tail.get(open.len_utf8()..)?;
    let main = head.trim_end();
    let sub = raw_sub.trim();
    if sub.is_empty() {
        return None;
    }
    Some((main, sub))
}
