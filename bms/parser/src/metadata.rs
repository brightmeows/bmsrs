//! Song-level metadata fields.
//!
//! Corresponds to the tokenizer's [`BmsHeaderMetadata`].

use std::collections::BTreeMap;

use bmsrs_bms_values::{BmsHeaderMetadata, TextIndex};

/// Song and chart identity metadata.
///
/// Every field uses last-wins semantics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// Song title (`#TITLE`).
    pub title: Option<String>,
    /// Song subtitle (`#SUBTITLE`).
    pub subtitle: Option<String>,
    /// Main artist / composer (`#ARTIST`).
    pub artist: Option<String>,
    /// Co-creators (`#SUBARTIST`).
    pub sub_artist: Option<String>,
    /// Music genre (`#GENRE` / `#GENLE`).
    pub genre: Option<String>,
    /// BMS chart author name (`#MAKER`).
    pub maker: Option<String>,
    /// Text shown in the song list (`#COMMENT`).
    pub comment: Option<String>,
    /// Character encoding hint (`#CHARSET`).
    pub charset: Option<String>,
    /// Author website URL (`%URL`).
    pub url: Option<String>,
    /// Author email address (`%EMAIL`).
    pub email: Option<String>,
    /// Timed on-screen text definitions (`#TEXTxx`, `#SONGxx`).
    pub text_defs: BTreeMap<TextIndex, String>,
}

impl Metadata {
    /// Apply one metadata header command to this structure.
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

    /// Parse an implicit subtitle from the title.
    ///
    /// The BMS specification allows a `#TITLE` value to imply a subtitle using
    /// separators. Priority (the first match wins): `-`, then `～`, then `()`,
    /// then `[]`, then `<>`.
    ///
    /// Does nothing when a subtitle has already been set explicitly through
    /// `#SUBTITLE`.
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

/// Parse a `sep...sep` suffix pattern (such as `-sub-` or `～sub～`).
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

/// Parse an `open...close` suffix pattern (such as `(sub)`, `[sub]`, or
/// `<sub>`).
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
