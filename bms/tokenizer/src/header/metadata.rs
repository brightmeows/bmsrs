//! `#maker`, `#subtitle`, `#url` and other file-level metadata.

/// Song/chart metadata headers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeaderMetadata<'a> {
    /// `#TITLE`
    #[serde(borrow)]
    Title(&'a str),
    /// `#SUBTITLE`
    #[serde(borrow)]
    Subtitle(&'a str),
    /// `#ARTIST`
    #[serde(borrow)]
    Artist(&'a str),
    /// `#SUBARTIST`
    #[serde(borrow)]
    SubArtist(&'a str),
    /// `#GENRE`
    #[serde(borrow)]
    Genre(&'a str),
    /// `#MAKER`
    #[serde(borrow)]
    Maker(&'a str),
    /// `#COMMENT`
    #[serde(borrow)]
    Comment(&'a str),
    /// `#TEXT` or `#SONG`
    #[serde(borrow)]
    Text(&'a str),
    /// `#CHARSET`
    #[serde(borrow)]
    Charset(&'a str),
    /// `%URL`
    #[serde(borrow)]
    Url(&'a str),
    /// `%EMAIL`
    #[serde(borrow)]
    Email(&'a str),
}
