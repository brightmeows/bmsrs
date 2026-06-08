//! `#maker`, `#subtitle`, `#url` and other file-level metadata.

/// Song/chart metadata headers.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsHeaderMetadata<'a> {
    /// `#TITLE`
    Title(&'a str),
    /// `#SUBTITLE`
    Subtitle(&'a str),
    /// `#ARTIST`
    Artist(&'a str),
    /// `#SUBARTIST`
    SubArtist(&'a str),
    /// `#GENRE`
    Genre(&'a str),
    /// `#MAKER`
    Maker(&'a str),
    /// `#COMMENT`
    Comment(&'a str),
    /// `#TEXT` or `#SONG`
    Text(&'a str),
    /// `#CHARSET`
    Charset(&'a str),
    /// `%URL`
    Url(&'a str),
    /// `%EMAIL`
    Email(&'a str),
}
