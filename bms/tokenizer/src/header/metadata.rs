//! Song/chart metadata headers: `#TITLE`, `#SUBTITLE`, `#ARTIST`,
//! `#SUBARTIST`, `#GENRE`, `#MAKER`, `#COMMENT`, `#TEXT`/`#SONG`,
//! `#CHARSET`, `%URL`, `%EMAIL`.

use crate::BmsTokenAttr;
use crate::index::{BmsIndex, TextTag};
use crate::{BmsHeader, BmsTryFromError};

/// Song/chart metadata headers.
///
/// These commands identify the chart and its authors.  They carry no
/// gameplay effect — they are purely informational.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderMetadata<'a> {
    /// `#TITLE` — song title.
    ///
    /// **Should not be omitted** — some players crash when it is absent
    /// (nanasi).  No length limit in the spec, but some players truncate
    /// or crash on very long titles (DDR: 500 byte limit).  May contain
    /// multi-byte characters depending on the file's encoding.
    #[bms_token("#TITLE {}")]
    Title(&'a str),
    /// `#SUBTITLE` — explicit subtitle (nanasi extension).
    ///
    /// Preferred over the legacy "implicit subtitle" parsing where
    /// delimiters (`-`, `~`, `()`, `[]`, `<>`) inside `#TITLE` are used
    /// to split the title.  Implicit subtitle handling varies by player.
    ///
    /// Multiple `#SUBTITLE` lines are supported by Sonorous.
    #[bms_token("#SUBTITLE {}")]
    Subtitle(&'a str),
    /// `#ARTIST` — song artist / composer.
    #[bms_token("#ARTIST {}")]
    Artist(&'a str),
    /// `#SUBARTIST` — co-creators (LR2 extension).
    ///
    /// Typically used for BGA authors, charter, etc.  Displayed
    /// differently from `#ARTIST` in supporting players.
    /// Multiple `#SUBARTIST` lines are supported by `TechnicalGroove`.
    #[bms_token("#SUBARTIST {}")]
    SubArtist(&'a str),
    /// `#GENRE` or `#GENLE` — music genre.
    ///
    /// `#GENLE` is a typo alias (uBMplay); both map to the same variant.
    /// Default when omitted: empty string.
    #[bms_token("#GENRE {}")]
    #[bms_token("#GENLE {}")]
    Genre(&'a str),
    /// `#MAKER` — BMS chart author name (bemaniaDX extension).
    ///
    /// Distinguishes the charter from the music composer.  Not displayed
    /// during gameplay — pure metadata.
    #[bms_token("#MAKER {}")]
    Maker(&'a str),
    /// `#COMMENT` — text shown in the song-selection list (pomu extension).
    ///
    /// May be wrapped in double quotes for empty strings, but parsers
    /// should not rely on the quotes being present (legacy charts omit
    /// them).  Multiple `#COMMENT` lines are supported by Sonorous.
    #[bms_token("#COMMENT {}")]
    Comment(&'a str),
    /// `#TEXT[00-ZZ]` or `#SONG[01-ZZ]` — timed on-screen text (pomu extension).
    ///
    /// Referenced by channel `#xxx99`.  `#TEXT00` is displayed on miss
    /// in nanasi.  `#SONG` is an obsolete alias — prefer `#TEXT`.
    ///
    /// The value may optionally be wrapped in double quotes, but parsers
    /// should not rely on the quotes being present.
    #[bms_token("#TEXT{id} {value}")]
    #[bms_token("#SONG{id} {value}")]
    Text {
        /// The 2-character index (e.g., `"00"`, `"aa"`).
        id: BmsIndex<TextTag>,
        /// The text content.
        value: &'a str,
    },
    /// `#CHARSET` — character encoding hint (ruvit extension, now obsolete).
    ///
    /// Values: `EUC-KR`, `SHIFT-JIS`, `UTF-8`.  Modern ruvit (2.0b5p2+)
    /// auto-detects encoding and ignores this command.  For new charts,
    /// save as UTF-8 (with or without BOM).
    #[bms_token("#CHARSET {}")]
    Charset(&'a str),
    /// `%URL` — author's website URL (BMS Manager extension).
    ///
    /// **Caveat**: BMSE and iBMSC delete `%URL` on save.
    #[bms_token("%URL {}")]
    Url(&'a str),
    /// `%EMAIL` — author's email address (BMS Manager extension).
    ///
    /// **Caveat**: BMSE and iBMSC delete `%EMAIL` on save.
    #[bms_token("%EMAIL {}")]
    Email(&'a str),
}

// From / TryFrom conversions

impl<'a> From<BmsHeaderMetadata<'a>> for BmsHeader<'a> {
    #[inline]
    fn from(meta: BmsHeaderMetadata<'a>) -> Self {
        BmsHeader::Metadata(meta)
    }
}

impl<'a> TryFrom<BmsHeader<'a>> for BmsHeaderMetadata<'a> {
    type Error = BmsTryFromError<'a>;

    #[inline]
    fn try_from(header: BmsHeader<'a>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Metadata(m) => Ok(m),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}
