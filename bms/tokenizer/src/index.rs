//! Type-safe BMS index wrapper backed by `[u8; 2]`.
//!
//! BMS uses 2-character indices to reference resources (WAV, BMP), timing
//! definitions (BPM, STOP, SCROLL, SPEED), channel numbers, and other
//! indexed commands.  The character set varies by context:
//!
//! - Most indexed commands use Base62 (`"01"`–`"zz"`, with `#BASE 62`).
//! - Standard resource indices use Base36 (`"01"`–`"ZZ"`).
//! - Channel data lines (`#xxxYY:values`) use hexadecimal (Base16).
//!
//! This module provides a typed wrapper [`BmsIndex<T, C>`] where `T`
//! prevents mixing index *kinds* and `C` constrains the *character set*
//! at compile time.
//!
//! # Storage
//!
//! The ID is stored as `[u8; 2]` where each byte is the raw ASCII character.
//! For single-character IDs (e.g., `"A"`), the second byte is `0`.

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use thiserror::Error;

use crate::IntoTokensError;

/// Character set constraint for [`BmsIndex`].
///
/// Implementations define which bytes are valid for a given index context.
pub trait BmsCharset {
    /// Returns `true` if `b` is a valid character in this charset.
    fn is_valid(b: u8) -> bool;
}

/// Base-62 charset: `0`–`9`, `A`–`Z`, `a`–`z`.
///
/// The default charset for most BMS indexed commands (WAV, BMP, BPM,
/// STOP, SCROLL, SPEED).  When `#BASE 62` is declared, all two-character
/// indices use this full range (3844 unique IDs); otherwise the
/// conventional range is base-36 `[0-9A-Z]` (1296 IDs), with lowercase
/// letters mapping to the same values as uppercase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Base62 {}

impl BmsCharset for Base62 {
    fn is_valid(b: u8) -> bool {
        matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
    }
}

/// Base-36 uppercase charset: `0`–`9`, `A`–`Z`.
///
/// Used by commands that only accept uppercase indices (e.g., some
/// older implementations).  The standard BMS range is `[01-ZZ]`,
/// giving 1296 unique two-character IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Base36 {}

impl BmsCharset for Base36 {
    fn is_valid(b: u8) -> bool {
        matches!(b, b'0'..=b'9' | b'A'..=b'Z')
    }
}

/// Base16 (hexadecimal) charset: `0`–`9`, `A`–`F`, `a`–`f`.
///
/// Used for channel numbers in message lines (`#xxxYY:values`), where
/// `YY` is a two-digit hex value (`00`–`FF`, 256 channels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Base16 {}

impl BmsCharset for Base16 {
    fn is_valid(b: u8) -> bool {
        b.is_ascii_hexdigit()
    }
}

/// A validated BMS 1–2 character index, stored as raw ASCII bytes.
///
/// `T` is a zero-size tag type (e.g., [`WavTag`], [`BpmTag`]) that prevents
/// cross-type misuse at compile time — a `BmsIndex<WavTag>` cannot be
/// used where a `BmsIndex<BmpTag>` is expected.
///
/// `C` is a charset constraint (default: [`Base62`]). For example,
/// `BmsIndex<ChannelTag, Base16>` only accepts hexadecimal characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BmsIndex<T, C: BmsCharset = Base62> {
    /// Raw ASCII bytes of the characters.
    ///
    /// # Invariant
    ///
    /// When the ID is a single character (e.g., `"A"`), `bytes[1]` is `0`.
    /// Two-character IDs store both bytes — neither is ever `0` because
    /// the charset validators exclude `NUL`.
    bytes: [u8; 2],
    /// Zero-sized tag and charset for compile-time safety.
    _phantom: PhantomData<(T, C)>,
}

impl<T, C: BmsCharset> fmt::Display for BmsIndex<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "guarded by match on bytes.len() or bytes[1] sentinel check"
)]
impl<T, C: BmsCharset> BmsIndex<T, C> {
    /// Return the ASCII string representation of this ID (borrows from self).
    #[must_use]
    pub fn as_str(&self) -> &str {
        let len = if self.bytes[1] == 0 { 1 } else { 2 };
        // SAFETY: bytes are validated as ASCII alphanumeric on construction.
        unsafe { std::str::from_utf8_unchecked(&self.bytes[..len]) }
    }

    /// Return the raw ASCII bytes (1 or 2 bytes).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        let len = if self.bytes[1] == 0 { 1 } else { 2 };
        &self.bytes[..len]
    }

    /// Convert the hexadecimal characters to a `u8` value.
    ///
    /// For a two-character ID like `"0A"` returns `10`.  For a
    /// single-character ID like `"A"` returns `10` (no shift).
    ///
    /// Returns `None` if either byte is not a valid hex character.
    #[must_use]
    pub fn as_u8_hex(&self) -> Option<u8> {
        let hi = hex_digit_value(self.bytes[0])?;
        if self.bytes[1] == 0 {
            Some(hi)
        } else {
            let lo = hex_digit_value(self.bytes[1])?;
            Some(hi << 4 | lo)
        }
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "guarded by match on bytes.len() or sentinel check"
)]
impl<T, C: BmsCharset> TryFrom<&str> for BmsIndex<T, C> {
    type Error = BmsIndexError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        match bytes.len() {
            1 if C::is_valid(bytes[0]) => Ok(Self {
                bytes: [bytes[0], 0],
                _phantom: PhantomData,
            }),
            2 if C::is_valid(bytes[0]) && C::is_valid(bytes[1]) => Ok(Self {
                bytes: [bytes[0], bytes[1]],
                _phantom: PhantomData,
            }),
            _ => Err(BmsIndexError {
                input: s.to_owned(),
            }),
        }
    }
}

/// Allows `value.parse::<BmsIndex<T, C>>()` with unified error conversion.
impl<T, C: BmsCharset> FromStr for BmsIndex<T, C> {
    type Err = BmsIndexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

/// Base-62 index methods for channel IDs.
impl<T> BmsIndex<T, Base62> {
    /// Convert the base-36 characters to a numeric index.
    ///
    /// `"00"` → `0`, `"ZZ"` → `1295`.  For a single-character ID like
    /// `"A"` returns `10` (no multiply).
    ///
    /// Returns `None` if the characters cannot be decoded (should not happen
    /// when the ID was constructed through [`TryFrom`]).
    #[must_use]
    pub fn to_index(&self) -> Option<u16> {
        let hi = base36_digit_value(self.bytes[0])?;
        if self.bytes[1] == 0 {
            Some(hi)
        } else {
            let lo = base36_digit_value(self.bytes[1])?;
            Some(hi * 36 + lo)
        }
    }
}

/// Decode a single hex ASCII byte to its numeric value (0–15).
fn hex_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Decode a single base-36 ASCII byte to its numeric value (0–35).
fn base36_digit_value(b: u8) -> Option<u16> {
    match b {
        b'0'..=b'9' => Some(u16::from(b - b'0')),
        b'A'..=b'Z' => Some(u16::from(b - b'A') + 10),
        b'a'..=b'z' => Some(u16::from(b - b'a') + 10),
        _ => None,
    }
}

/// Error returned when a channel ID string is not a valid 1–2 character
/// sequence for the specified charset.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid BMS index: {input}")]
pub struct BmsIndexError {
    /// The raw string that failed validation.
    pub input: String,
}

impl<'a> IntoTokensError<'a> for BmsIndexError {
    fn into_error(self, _context: &'static str, value: &'a str) -> crate::BmsTokenizeError<'a> {
        crate::BmsTokenizeError::InvalidInteger { value }
    }
}

// Tag types — zero-sized, never instantiated.
// These exist purely at the type level to prevent mixing different
// kinds of indexed commands at compile time.

/// Tag type for channel numbers in message lines (`#xxxYY:values`).
///
/// Channel IDs are hexadecimal (`00`–`FF`) and represent which "lane"
/// or "function" the objects belong to (BGM, visible notes, invisible
/// notes, long notes, BPM changes, stops, landmines, etc.).
/// See the BMS command memo channel-mapping table for the full list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelTag {}
/// Tag type for `#WAVxx` / `#EXWAVxx` index IDs.
///
/// Sound definitions.  `00` is special: it defines the landmine
/// explosion sound (channels `#xxxD1-E9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WavTag {}
/// Tag type for `#BMPxx` / `#EXBMPxx` / `#BGAxx` / `#@BGAxx` /
/// `#SWBGAxx` / `#ARGBxx` index IDs.
///
/// Image / BGA definitions.  `00` is the default miss/poor image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BmpTag {}
/// Tag type for `#BPMxx` / `#EXBPMxx` index IDs.
///
/// Extended BPM definitions, referenced by channel `#xxx08`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BpmTag {}
/// Tag type for `#STOPxx` index IDs.
///
/// Stop-sequence definitions, referenced by channel `#xxx09`.
/// Values are in 192nd-note units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StopTag {}
/// Tag type for `#SCROLLxx` index IDs.
///
/// Scroll speed multiplier definitions, referenced by channel `#xxxSC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScrollTag {}
/// Tag type for `#SPEEDxx` index IDs.
///
/// Visual note-spacing definitions, referenced by channel `#xxxSP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpeedTag {}
/// Tag type for `#EXRANKxx` index IDs.
///
/// Per-position judgment width overrides, referenced by channel
/// `#xxxA0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExRankTag {}
/// Tag type for `#SEEKxx` index IDs.
///
/// Video seek positions, referenced by channel `#xxx05`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeekTag {}
/// Tag type for `#LNOBJ` value (a WAV index used as LN terminator).
///
/// The value references a `#WAV` index — when a note with this index
/// appears on channels `#xxx11-29`, it marks the end of a long note.
/// **Recommendation**: use uppercase to avoid nanasi/fgt++ lowercase
/// recognition bugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LnObjTag {}
/// Tag type for `#TEXT[00-ZZ]` / `#SONG[01-ZZ]` index IDs.
///
/// Timed on-screen text definitions, referenced by channel `#xxx99`.
/// `#TEXT00` is the miss text shown on poor judgment in nanasi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextTag {}
/// Tag type for `#CHANGEOPTION[01-ZZ]` index IDs.
///
/// Dynamic option-change definitions, referenced by channel `#xxxA6`.
/// Used to change player options mid-chart (nanasi extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeOptionTag {}

/// Tag type for 2-character object indices in message body values
/// (the string after `:` in `#xxxYY:values`).
///
/// These indices reference resource definitions (WAV, BMP, BPM, etc.)
/// and are parsed leniently — invalid characters are skipped, and every
/// 2 consecutive valid Base62 characters form an object ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectTag {}

/// A validated 2-character object index from a message body.
///
/// This is a type alias for [`BmsIndex`] parameterized with [`ObjectTag`]
/// and the [`Base62`] charset, used for the leniently parsed object IDs
/// in [`BmsMessage`](crate::BmsMessage).
pub type BmsObjectId = BmsIndex<ObjectTag, Base62>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base62_valid_two_char_ids() {
        assert!(BmsIndex::<WavTag, Base62>::try_from("01").is_ok());
        assert!(BmsIndex::<WavTag>::try_from("ZZ").is_ok());
        assert!(BmsIndex::<WavTag>::try_from("aa").is_ok());
        assert!(BmsIndex::<WavTag>::try_from("zZ").is_ok());
        assert!(BmsIndex::<WavTag>::try_from("FF").is_ok());
    }

    #[test]
    fn base62_valid_one_char_ids() {
        assert!(BmsIndex::<WavTag>::try_from("A").is_ok());
        assert!(BmsIndex::<WavTag>::try_from("9").is_ok());
        assert!(BmsIndex::<WavTag>::try_from("0").is_ok());
    }

    #[test]
    fn base62_empty_id_is_invalid() {
        assert!(BmsIndex::<WavTag>::try_from("").is_err());
    }

    #[test]
    fn base62_three_char_id_is_invalid() {
        assert!(BmsIndex::<WavTag>::try_from("AAA").is_err());
    }

    #[test]
    fn base62_non_alphanumeric_is_invalid() {
        assert!(BmsIndex::<WavTag>::try_from("*!").is_err());
        assert!(BmsIndex::<WavTag>::try_from(" ").is_err());
        assert!(BmsIndex::<WavTag>::try_from("ab:").is_err());
        assert!(BmsIndex::<WavTag>::try_from("-1").is_err());
    }

    #[test]
    fn hex_valid_ids() {
        assert!(BmsIndex::<ChannelTag, Base16>::try_from("0A").is_ok());
        assert!(BmsIndex::<ChannelTag, Base16>::try_from("FF").is_ok());
        assert!(BmsIndex::<ChannelTag, Base16>::try_from("D1").is_ok());
        assert!(BmsIndex::<ChannelTag, Base16>::try_from("ff").is_ok());
    }

    #[test]
    fn hex_rejects_non_hex() {
        assert!(BmsIndex::<ChannelTag, Base16>::try_from("GZ").is_err());
        assert!(BmsIndex::<ChannelTag, Base16>::try_from("ZZ").is_err());
        assert!(BmsIndex::<ChannelTag, Base16>::try_from("gh").is_err());
    }

    #[test]
    fn base36_upper_valid_ids() {
        assert!(BmsIndex::<WavTag, Base36>::try_from("01").is_ok());
        assert!(BmsIndex::<WavTag, Base36>::try_from("ZZ").is_ok());
        assert!(BmsIndex::<WavTag, Base36>::try_from("A0").is_ok());
    }

    #[test]
    fn base36_upper_rejects_lowercase() {
        assert!(BmsIndex::<WavTag, Base36>::try_from("aa").is_err());
        assert!(BmsIndex::<WavTag, Base36>::try_from("zZ").is_err());
    }

    #[test]
    fn different_tags_prevent_mixing() {
        let wav_id = BmsIndex::<WavTag>::try_from("01").unwrap();
        let bmp_id = BmsIndex::<BmpTag>::try_from("01").unwrap();
        assert_eq!(wav_id.as_str(), bmp_id.as_str());
    }

    #[test]
    fn as_str_returns_original() {
        let id = BmsIndex::<WavTag>::try_from("2A").unwrap();
        assert_eq!(id.as_str(), "2A");
    }

    #[test]
    fn as_str_one_char() {
        let id = BmsIndex::<WavTag>::try_from("A").unwrap();
        assert_eq!(id.as_str(), "A");
    }

    #[test]
    fn as_bytes_two_char() {
        let id = BmsIndex::<WavTag>::try_from("2A").unwrap();
        assert_eq!(id.as_bytes(), &[b'2', b'A']);
    }

    #[test]
    fn as_bytes_one_char() {
        let id = BmsIndex::<WavTag>::try_from("A").unwrap();
        assert_eq!(id.as_bytes(), &[b'A']);
    }

    #[test]
    fn display_output() {
        let id = BmsIndex::<WavTag>::try_from("FF").unwrap();
        assert_eq!(id.to_string(), "FF");
    }

    #[test]
    fn bms_channel_id_error_display() {
        let err = BmsIndexError {
            input: "!!!".to_string(),
        };
        assert!(err.to_string().contains("!!!"));
    }

    #[test]
    fn tag_types_are_zero_sized() {
        assert_eq!(std::mem::size_of::<WavTag>(), 0);
        assert_eq!(std::mem::size_of::<BmpTag>(), 0);
        assert_eq!(std::mem::size_of::<BpmTag>(), 0);
        assert_eq!(std::mem::size_of::<ChannelTag>(), 0);
    }

    #[test]
    fn size_of_bms_channel_id() {
        // [u8; 2] + PhantomData<(T, C)> → 2 bytes, no pointer overhead.
        assert_eq!(std::mem::size_of::<BmsIndex<WavTag>>(), 2);
        assert_eq!(std::mem::size_of::<BmsIndex<WavTag, Base62>>(), 2);
        assert_eq!(std::mem::size_of::<BmsIndex<ChannelTag, Base16>>(), 2);
    }

    #[test]
    fn as_u8_hex_two_digits() {
        let id = BmsIndex::<ChannelTag, Base16>::try_from("0A").unwrap();
        assert_eq!(id.as_u8_hex(), Some(10));
        let id = BmsIndex::<ChannelTag, Base16>::try_from("FF").unwrap();
        assert_eq!(id.as_u8_hex(), Some(255));
        let id = BmsIndex::<ChannelTag, Base16>::try_from("D1").unwrap();
        assert_eq!(id.as_u8_hex(), Some(209));
    }

    #[test]
    fn as_u8_hex_one_digit() {
        let id = BmsIndex::<ChannelTag, Base16>::try_from("A").unwrap();
        assert_eq!(id.as_u8_hex(), Some(10));
        let id = BmsIndex::<ChannelTag, Base16>::try_from("f").unwrap();
        assert_eq!(id.as_u8_hex(), Some(15));
    }

    #[test]
    fn to_index_base62() {
        let id = BmsIndex::<WavTag>::try_from("00").unwrap();
        assert_eq!(id.to_index(), Some(0));
        let id = BmsIndex::<WavTag>::try_from("01").unwrap();
        assert_eq!(id.to_index(), Some(1));
        let id = BmsIndex::<WavTag>::try_from("ZZ").unwrap();
        assert_eq!(id.to_index(), Some(1295));
        let id = BmsIndex::<WavTag>::try_from("0A").unwrap();
        assert_eq!(id.to_index(), Some(10));
    }

    #[test]
    fn to_index_single_char() {
        let id = BmsIndex::<WavTag>::try_from("A").unwrap();
        assert_eq!(id.to_index(), Some(10));
        let id = BmsIndex::<WavTag>::try_from("z").unwrap();
        assert_eq!(id.to_index(), Some(35));
    }
}
