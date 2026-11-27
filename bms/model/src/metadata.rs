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
}
