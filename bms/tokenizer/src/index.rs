//! Type-safe BMS index wrappers.
//!
//! BMS uses 1–2 character indices to reference resources (WAV, BMP),
//! timing definitions (BPM, STOP, SCROLL, SPEED), channel numbers, and
//! other indexed commands.
//!
//! [`BmsIndex`] is the raw storage type with Base62 character validation.
//! Each semantic kind of index is a newtype wrapper around `BmsIndex`
//! (e.g., [`WavIndex`], [`BpmIndex`]) providing compile-time type safety.
//!
//! # Character sets
//!
//! Most indices accept Base62 characters (`0-9A-Za-z`). The [`ChannelIndex`]
//! type is special — it validates as Base36 (`0-9A-Z`) because channel
//! numbers in BMS message lines use hexadecimal.
//!
//! The [`BmsBase`] enum and [`BmsIndex::is_valid_for`] method enable
//! runtime charset checking when needed.
//!
//! # Storage
//!
//! The ID is stored as `[u8; 2]` where each byte is the raw ASCII character.
//! For single-character IDs (e.g., `"A"`), the second byte is `0`.

use std::fmt;
use std::str::FromStr;

use derive_more::{Deref, Display, From, FromStr};
use thiserror::Error;

use crate::IntoTokensError;

// Charset check helpers

/// Base-62 chars: `0`–`9`, `A`–`Z`, `a`–`z`.
#[inline]
pub const fn is_base62(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
}

/// Base-36 uppercase chars: `0`–`9`, `A`–`Z`.
#[inline]
const fn is_base36(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z')
}

/// Base-16 hex chars: `0`–`9`, `A`–`F`, `a`–`f`.
#[inline]
const fn is_base16(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

// BmsBase — runtime charset enum

/// Character set for BMS index validation.
///
/// This replaces the previous compile-time `BmsCharset` type parameter.
/// Use [`BmsIndex::is_valid_for`] for runtime checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BmsBase {
    /// Hexadecimal (`0`–`9`, `A`–`F`, `a`–`f`; 16 values per position).
    Base16,
    /// Base-36 uppercase (`0`–`9`, `A`–`Z`; 36 values per position).
    Base36,
    /// Base-62 (`0`–`9`, `A`–`Z`, `a`–`z`; 62 values per position).
    /// This is the default charset for most BMS indices.
    Base62,
}

// BmsIndex — raw storage (no generics)

/// A validated 1–2 character BMS index, stored as raw ASCII bytes.
///
/// Construction via [`TryFrom<&str>`] / [`FromStr`] validates with Base62
/// (`0-9A-Za-z`), the most permissive charset.  See [`BmsBase`] and
/// [`is_valid_for`](Self::is_valid_for) for runtime charset checking.
///
/// # Storage
///
/// `[u8; 2]` — when the ID is a single character, `bytes[1]` is `0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BmsIndex {
    /// Raw ASCII bytes of the characters.
    ///
    /// # Invariant
    ///
    /// When the ID is a single character (e.g., `"A"`), `bytes[1]` is `0`.
    /// Two-character IDs store both bytes — neither is ever `0` because
    /// all charset validators exclude `NUL`.
    bytes: [u8; 2],
}

#[expect(
    clippy::indexing_slicing,
    reason = "guarded by match on bytes.len() or bytes[1] sentinel check"
)]
impl BmsIndex {
    /// Return the ASCII string representation of this ID (borrows from self).
    #[must_use]
    #[expect(unsafe_code, reason = "ASCII bytes validated on construction")]
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

    /// Convert the base-36 characters to a numeric index.
    ///
    /// `"00"` → `0`, `"ZZ"` → `1295`.  For a single-character ID like
    /// `"A"` returns `10` (no multiply).
    ///
    /// Returns `None` if the characters cannot be decoded (should not happen
    /// when the ID was constructed through [`TryFrom`] / [`FromStr`]).
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
            Some((hi << 4) | lo)
        }
    }

    /// Check whether all characters in this index are valid for the given
    /// character set.
    #[must_use]
    pub fn is_valid_for(&self, base: BmsBase) -> bool {
        let check: fn(u8) -> bool = match base {
            BmsBase::Base16 => is_base16,
            BmsBase::Base36 => is_base36,
            BmsBase::Base62 => is_base62,
        };
        check(self.bytes[0]) && (self.bytes[1] == 0 || check(self.bytes[1]))
    }

    /// Create from already-validated bytes (internal use by newtypes).
    #[inline]
    #[must_use]
    const fn from_valid(bytes: [u8; 2]) -> Self {
        Self { bytes }
    }
}

impl fmt::Display for BmsIndex {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[expect(clippy::indexing_slicing, reason = "guarded by match on bytes.len()")]
impl TryFrom<&str> for BmsIndex {
    type Error = BmsIndexError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        match bytes.len() {
            1 if is_base62(bytes[0]) => Ok(Self {
                bytes: [bytes[0], 0],
            }),
            2 if is_base62(bytes[0]) && is_base62(bytes[1]) => Ok(Self {
                bytes: [bytes[0], bytes[1]],
            }),
            _ => Err(BmsIndexError {
                input: s.to_owned(),
            }),
        }
    }
}

impl FromStr for BmsIndex {
    type Err = BmsIndexError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

// Digit decoding helpers

/// Decode a single hex ASCII byte to its numeric value (0–15).
const fn hex_digit_value(b: u8) -> Option<u8> {
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

// Error type

/// Error returned when a BMS index string is not a valid 1–2 character
/// sequence for the required charset.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid BMS index: {input}")]
pub struct BmsIndexError {
    /// The raw string that failed validation.
    pub input: String,
}

impl<C> IntoTokensError<C> for BmsIndexError {
    #[inline]
    fn into_error(self, _context: &'static str, value: C) -> crate::BmsTokenizeError<C> {
        crate::BmsTokenizeError::InvalidInteger { value }
    }
}

// Newtype wrappers — one per semantic index kind

// Each newtype:
//   - wraps BmsIndex
//   - provides Deref<Target = BmsIndex> for transparent method access
//   - validates its required charset on construction
//   - implements Display, FromStr, TryFrom<&str>, Clone, Copy, etc.

/// Generate a `TryFrom<&str>` impl that delegates to `FromStr`.
macro_rules! impl_try_from_str {
    ($ty:ty) => {
        impl TryFrom<&str> for $ty {
            type Error = BmsIndexError;

            #[inline]
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

// Standard Base62 newtypes

/// Index for `#WAV{id}` / `#EXWAV{id}` — sound definition references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct WavIndex(pub BmsIndex);
impl_try_from_str!(WavIndex);

/// Index for `#BMP{id}` / `#BGA{id}` / `#@BGA{id}` / `#SWBGA{id}` /
/// `#ARGB{id}` / `#EXBMP{id}` — image / BGA definition references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct BmpIndex(pub BmsIndex);
impl_try_from_str!(BmpIndex);

/// Index for `#BPM{id}` / `#EXBPM{id}` — extended BPM definition references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct BpmIndex(pub BmsIndex);
impl_try_from_str!(BpmIndex);

/// Index for `#STOP{id}` — stop definition references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct StopIndex(pub BmsIndex);
impl_try_from_str!(StopIndex);

/// Index for `#SCROLL{id}` — scroll speed definition references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ScrollIndex(pub BmsIndex);
impl_try_from_str!(ScrollIndex);

/// Index for `#SPEED{id}` — speed/spacing definition references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct SpeedIndex(pub BmsIndex);
impl_try_from_str!(SpeedIndex);

/// Index for `#EXRANK{id}` — per-position judgment override references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ExRankIndex(pub BmsIndex);
impl_try_from_str!(ExRankIndex);

/// Index for `#SEEK{id}` — video seek position references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct SeekIndex(pub BmsIndex);
impl_try_from_str!(SeekIndex);

/// Index for `#LNOBJ` — WAV index used as LN terminator marker.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct LnObjIndex(pub BmsIndex);
impl_try_from_str!(LnObjIndex);

/// Index for `#TEXT{id}` / `#SONG{id}` — timed on-screen text references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct TextIndex(pub BmsIndex);
impl_try_from_str!(TextIndex);

/// Index for `#CHANGEOPTION{id}` — dynamic option-change references.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ChangeOptionIndex(pub BmsIndex);
impl_try_from_str!(ChangeOptionIndex);

/// Index for 2-character object IDs in message body values.
///
/// These are the leniently-parsed object indices in [`BmsMessage`](crate::BmsMessage)
/// body strings — every consecutive pair of valid Base62 characters forms
/// an object ID.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ObjectIndex(pub BmsIndex);
impl_try_from_str!(ObjectIndex);

// Special newtype: ChannelIndex (Base36 validation)

/// Channel number in BMS message lines (`#xxxYY:values`).
///
/// Validates as Base36 (`0-9A-Z`) instead of the default Base62.
/// Use [`as_u8_hex`](BmsIndex::as_u8_hex) to convert to a byte value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display)]
#[display("{}", _0)]
pub struct ChannelIndex(pub BmsIndex);

#[expect(clippy::indexing_slicing, reason = "guarded by match on bytes.len()")]
impl FromStr for ChannelIndex {
    type Err = BmsIndexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        match bytes.len() {
            1 if is_base36(bytes[0]) => Ok(Self(BmsIndex::from_valid([bytes[0], 0]))),
            2 if is_base36(bytes[0]) && is_base36(bytes[1]) => {
                Ok(Self(BmsIndex::from_valid([bytes[0], bytes[1]])))
            }
            _ => Err(BmsIndexError {
                input: s.to_owned(),
            }),
        }
    }
}

impl_try_from_str!(ChannelIndex);

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // BmsIndex tests

    #[test]
    fn bms_index_valid_two_char() {
        assert!(BmsIndex::try_from("01").is_ok());
        assert!(BmsIndex::try_from("ZZ").is_ok());
        assert!(BmsIndex::try_from("aa").is_ok());
        assert!(BmsIndex::try_from("zZ").is_ok());
        assert!(BmsIndex::try_from("FF").is_ok());
    }

    #[test]
    fn bms_index_valid_one_char() {
        assert!(BmsIndex::try_from("A").is_ok());
        assert!(BmsIndex::try_from("9").is_ok());
        assert!(BmsIndex::try_from("0").is_ok());
    }

    #[test]
    fn bms_index_empty_is_invalid() {
        assert!(BmsIndex::try_from("").is_err());
    }

    #[test]
    fn bms_index_three_chars_is_invalid() {
        assert!(BmsIndex::try_from("AAA").is_err());
    }

    #[test]
    fn bms_index_non_alphanumeric_is_invalid() {
        assert!(BmsIndex::try_from("*!").is_err());
        assert!(BmsIndex::try_from(" ").is_err());
        assert!(BmsIndex::try_from("ab:").is_err());
        assert!(BmsIndex::try_from("-1").is_err());
    }

    #[test]
    fn as_str_returns_original() {
        let id = BmsIndex::try_from("2A").unwrap();
        assert_eq!(id.as_str(), "2A");
    }

    #[test]
    fn as_str_one_char() {
        let id = BmsIndex::try_from("A").unwrap();
        assert_eq!(id.as_str(), "A");
    }

    #[test]
    fn as_bytes_two_char() {
        let id = BmsIndex::try_from("2A").unwrap();
        assert_eq!(id.as_bytes(), b"2A");
    }

    #[test]
    fn as_bytes_one_char() {
        let id = BmsIndex::try_from("A").unwrap();
        assert_eq!(id.as_bytes(), b"A");
    }

    #[test]
    fn display_output() {
        let id = BmsIndex::try_from("FF").unwrap();
        assert_eq!(id.to_string(), "FF");
    }

    #[test]
    fn to_index_base62() {
        assert_eq!(BmsIndex::try_from("00").unwrap().to_index(), Some(0));
        assert_eq!(BmsIndex::try_from("01").unwrap().to_index(), Some(1));
        assert_eq!(BmsIndex::try_from("ZZ").unwrap().to_index(), Some(1295));
        assert_eq!(BmsIndex::try_from("0A").unwrap().to_index(), Some(10));
    }

    #[test]
    fn to_index_single_char() {
        assert_eq!(BmsIndex::try_from("A").unwrap().to_index(), Some(10));
        assert_eq!(BmsIndex::try_from("z").unwrap().to_index(), Some(35));
    }

    #[test]
    fn as_u8_hex_two_digits() {
        assert_eq!(BmsIndex::try_from("0A").unwrap().as_u8_hex(), Some(10));
        assert_eq!(BmsIndex::try_from("FF").unwrap().as_u8_hex(), Some(255));
        assert_eq!(BmsIndex::try_from("D1").unwrap().as_u8_hex(), Some(209));
    }

    #[test]
    fn as_u8_hex_one_digit() {
        assert_eq!(BmsIndex::try_from("A").unwrap().as_u8_hex(), Some(10));
        assert_eq!(BmsIndex::try_from("f").unwrap().as_u8_hex(), Some(15));
    }

    #[test]
    fn size_of_bms_index() {
        assert_eq!(std::mem::size_of::<BmsIndex>(), 2);
    }

    // BmsBase tests

    #[test]
    fn is_valid_for_base62() {
        let id = BmsIndex::try_from("aZ").unwrap();
        assert!(id.is_valid_for(BmsBase::Base62));
        assert!(!id.is_valid_for(BmsBase::Base36));
        assert!(!id.is_valid_for(BmsBase::Base16));
    }

    #[test]
    fn is_valid_for_base36() {
        let id = BmsIndex::try_from("AZ").unwrap();
        assert!(id.is_valid_for(BmsBase::Base62));
        assert!(id.is_valid_for(BmsBase::Base36));
        assert!(!id.is_valid_for(BmsBase::Base16));
    }

    #[test]
    fn is_valid_for_base16() {
        let id = BmsIndex::try_from("AF").unwrap();
        assert!(id.is_valid_for(BmsBase::Base62));
        assert!(id.is_valid_for(BmsBase::Base36));
        assert!(id.is_valid_for(BmsBase::Base16));
    }

    // Newtype tests

    #[test]
    fn newtypes_are_distinct_types() {
        let wav: WavIndex = "01".parse().unwrap();
        let bmp: BmpIndex = "01".parse().unwrap();
        // Same string, same underlying value, different types.
        assert_eq!(wav.as_str(), bmp.as_str());
        // This line would not compile:
        // let _: WavIndex = bmp;
    }

    #[test]
    fn wav_index_valid() {
        assert!(WavIndex::try_from("01").is_ok());
        assert!(WavIndex::try_from("aZ").is_ok());
    }

    #[test]
    fn wav_index_invalid() {
        assert!(WavIndex::try_from("").is_err());
        assert!(WavIndex::try_from("AAA").is_err());
    }

    #[test]
    fn channel_index_valid() {
        assert!(ChannelIndex::try_from("0A").is_ok());
        assert!(ChannelIndex::try_from("FF").is_ok());
        assert!(ChannelIndex::try_from("D1").is_ok());
    }

    #[test]
    fn channel_index_rejects_lowercase() {
        assert!(ChannelIndex::try_from("aa").is_err());
        assert!(ChannelIndex::try_from("zZ").is_err());
        assert!(ChannelIndex::try_from("ff").is_err());
        assert!(ChannelIndex::try_from("gh").is_err());
    }

    #[test]
    fn channel_index_as_u8_hex() {
        assert_eq!("0A".parse::<ChannelIndex>().unwrap().as_u8_hex(), Some(10));
        assert_eq!("FF".parse::<ChannelIndex>().unwrap().as_u8_hex(), Some(255));
    }

    #[test]
    fn newtype_size_is_2_bytes() {
        assert_eq!(std::mem::size_of::<WavIndex>(), 2);
        assert_eq!(std::mem::size_of::<BmpIndex>(), 2);
        assert_eq!(std::mem::size_of::<ChannelIndex>(), 2);
    }

    #[test]
    fn error_display() {
        let err = BmsIndexError {
            input: "!!!".to_owned(),
        };
        assert!(err.to_string().contains("!!!"));
    }

    #[test]
    fn deref_provides_bms_index_methods() {
        let wav: WavIndex = "2A".parse().unwrap();
        // Via Deref<Target = BmsIndex>
        assert_eq!(wav.as_str(), "2A");
        assert_eq!(wav.as_bytes(), b"2A");
        assert_eq!(wav.to_string(), "2A");
        assert_eq!(wav.to_index(), Some(82));
    }

    #[test]
    fn object_index_replaces_bms_object_id() {
        let obj: ObjectIndex = "ZZ".parse().unwrap();
        assert_eq!(obj.as_str(), "ZZ");
    }
}
