//! Song and chart metadata headers: `#TITLE`, `#SUBTITLE`, `#ARTIST`,
//! `#SUBARTIST`, `#GENRE`, `#MAKER`, `#COMMENT`, `#TEXT`/`#SONG`, `#CHARSET`,
//! `%URL`, and `%EMAIL`.

use crate::BmsTokenAttr;
use crate::index::TextIndex;

/// Song and chart metadata headers.
///
/// These commands identify the chart and its authors. They carry no gameplay
/// effect; they are purely informational.
#[derive(Debug, Clone, PartialEq, Eq, BmsTokenAttr)]
pub enum BmsHeaderMetadata {
    /// `#TITLE`: song title.
    ///
    /// **Should not be omitted**: some players crash when it is missing
    /// (nanasi). The specification sets no length limit, but some players
    /// truncate or crash on overly long titles (DDR: 500-byte limit). It may
    /// contain multi-byte characters depending on the file encoding.
    #[bms_token("#TITLE {}")]
    Title(String),
    /// `#SUBTITLE`: explicit subtitle (nanasi extension).
    ///
    /// Preferred over traditional "implicit subtitle" parsing, which splits the
    /// title using separators inside `#TITLE` (`-`, `~`, `()`, `[]`, `<>`).
    /// Implicit subtitle handling varies by player.
    ///
    /// Sonorous supports multi-line `#SUBTITLE`.
    #[bms_token("#SUBTITLE {}")]
    Subtitle(String),
    /// `#ARTIST`: song artist / composer.
    #[bms_token("#ARTIST {}")]
    Artist(String),
    /// `#SUBARTIST`: co-author (LR2 extension).
    ///
    /// Typically used for BGA authors, charters, and so on. In players that
    /// support it, it is displayed differently from `#ARTIST`.
    /// `TechnicalGroove` supports multi-line `#SUBARTIST`.
    #[bms_token("#SUBARTIST {}")]
    SubArtist(String),
    /// `#GENRE` or `#GENLE`: music genre.
    ///
    /// `#GENLE` is a misspelling alias (uBMplay); both map to the same variant.
    /// Default when omitted: the empty string.
    #[bms_token("#GENRE {}")]
    #[bms_token("#GENLE {}")]
    Genre(String),
    /// `#MAKER`: BMS chart author name (bemaniaDX extension).
    ///
    /// Distinguishes the charter from the music composer. Not shown during
    /// gameplay; pure metadata.
    #[bms_token("#MAKER {}")]
    Maker(String),
    /// `#COMMENT`: text shown in the song list (pomu extension).
    ///
    /// It may be wrapped in double quotes to represent an empty string, but
    /// parsers should not rely on the quotes being present (older charts omit
    /// them). Sonorous supports multi-line `#COMMENT`.
    #[bms_token("#COMMENT {}")]
    Comment(String),
    /// `#TEXT[00-ZZ]` or `#SONG[01-ZZ]`: timed on-screen text (pomu extension).
    ///
    /// Referenced by channel `#xxx99`. nanasi shows `#TEXT00` on a miss.
    /// `#SONG` is an obsolete alias; prefer `#TEXT`.
    ///
    /// The value may optionally be wrapped in double quotes, but parsers should
    /// not rely on the quotes being present.
    #[bms_token("#TEXT{id} {value}")]
    #[bms_token("#SONG{id} {value}")]
    Text {
        /// 2-character index (for example, `"00"`, `"aa"`).
        id: TextIndex,
        /// Text content.
        value: String,
    },
    /// `#CHARSET`: character encoding hint (ruvit extension, now deprecated).
    ///
    /// Values: `EUC-KR`, `SHIFT-JIS`, `UTF-8`. Modern ruvit (2.0b5p2+) detects
    /// the encoding automatically and ignores this command. For new charts,
    /// save as UTF-8 (with or without a BOM).
    #[bms_token("#CHARSET {}")]
    Charset(String),
    /// `%URL`: author website URL (BMS Manager extension).
    ///
    /// **Note**: BMSE and iBMSC delete `%URL` when saving.
    #[bms_token("%URL {}")]
    Url(String),
    /// `%EMAIL`: author email address (BMS Manager extension).
    ///
    /// **Note**: BMSE and iBMSC delete `%EMAIL` when saving.
    #[bms_token("%EMAIL {}")]
    Email(String),
}
