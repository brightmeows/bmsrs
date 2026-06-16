//! Song-level metadata fields.
//!
//! Corresponds to [`BmsHeaderMetadata`] from the tokenizer.

use std::collections::BTreeMap;

use bms_tokenizer::{BmsHeaderMetadata, BmsIndex, TextTag};

/// Song / chart identification metadata.
///
/// All fields use last-wins semantics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    /// Song title (`#TITLE`).
    pub title: Option<String>,
    /// Song subtitle (`#SUBTITLE`).
    pub subtitle: Option<String>,
    /// Primary artist / composer (`#ARTIST`).
    pub artist: Option<String>,
    /// Co-creators (`#SUBARTIST`).
    pub sub_artist: Option<String>,
    /// Music genre (`#GENRE` / `#GENLE`).
    pub genre: Option<String>,
    /// BMS chart author name (`#MAKER`).
    pub maker: Option<String>,
    /// Text shown in song-selection list (`#COMMENT`).
    pub comment: Option<String>,
    /// Character encoding hint (`#CHARSET`).
    pub charset: Option<String>,
    /// Author's website URL (`%URL`).
    pub url: Option<String>,
    /// Author's email address (`%EMAIL`).
    pub email: Option<String>,
    /// Timed on-screen text definitions (`#TEXTxx`, `#SONGxx`).
    pub text_defs: BTreeMap<BmsIndex<TextTag>, String>,
}

impl Metadata {
    /// Apply a metadata header to this struct.
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
