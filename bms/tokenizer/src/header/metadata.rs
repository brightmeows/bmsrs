//! `#maker`, `#subtitle`, `#url` and other file-level metadata.

use crate::BmsTokenAttr;

/// Song/chart metadata headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderMetadata<'a> {
    /// `#TITLE`
    #[bms_token("#TITLE {value}")]
    Title(&'a str),
    /// `#SUBTITLE`
    #[bms_token("#SUBTITLE {value}")]
    Subtitle(&'a str),
    /// `#ARTIST`
    #[bms_token("#ARTIST {value}")]
    Artist(&'a str),
    /// `#SUBARTIST`
    #[bms_token("#SUBARTIST {value}")]
    SubArtist(&'a str),
    /// `#GENRE` or `#GENLE`
    #[bms_token("#GENRE {value}")]
    #[bms_token("#GENLE {value}")]
    Genre(&'a str),
    /// `#MAKER`
    #[bms_token("#MAKER {value}")]
    Maker(&'a str),
    /// `#COMMENT`
    #[bms_token("#COMMENT {value}")]
    Comment(&'a str),
    /// `#TEXT` or `#SONG`
    #[bms_token("#TEXT {value}")]
    #[bms_token("#SONG {value}")]
    Text(&'a str),
    /// `#CHARSET`
    #[bms_token("#CHARSET {value}")]
    Charset(&'a str),
    /// `%URL`
    #[bms_token("%URL {value}")]
    Url(&'a str),
    /// `%EMAIL`
    #[bms_token("%EMAIL {value}")]
    Email(&'a str),
}
