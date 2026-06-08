//! Type-safe channel ID wrapper backed by `[u8; 2]`.
//!
//! BMS uses 2-character base-36 indices (`"01"`–`"ZZ"`, `"aa"`–`"zz"`) to
//! reference resources (WAV, BMP), timing definitions (BPM, STOP, SCROLL, SPEED),
//! and other indexed commands. This module provides a typed wrapper that
//! prevents accidentally mixing different index types at compile time.
//!
//! # Storage
//!
//! The ID is stored as `[u8; 2]` where each byte is the raw ASCII character.
//! For single-character IDs (e.g., `"A"`), the second byte is `0`.

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use crate::error::BmsTokenizeError;

/// Returns `true` if the byte is a BMS base-36 character:
/// `0`–`9`, `A`–`Z`, or `a`–`z`.
/// This is the strict BMS channel ID character set.
#[must_use]
fn is_bms_alphanumeric(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
}

/// A validated BMS channel ID (1–2 characters from `0`–`9`, `A`–`Z`, `a`–`z`),
/// stored as raw ASCII bytes.
///
/// `T` is a zero-size tag type (e.g., [`WavTag`], [`BpmTag`]) that prevents
/// cross-type misuse at compile time — a `BmsChannelId<WavTag>` cannot be
/// used where a `BmsChannelId<BmpTag>` is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BmsChannelId<T> {
    /// Raw ASCII bytes of the characters.
    ///
    /// # Invariant
    ///
    /// When the ID is a single character (e.g., `"A"`), `bytes[1]` is `0`.
    /// Two-character IDs store both bytes — neither is ever `0` because
    /// `is_bms_alphanumeric` excludes `NUL`.
    bytes: [u8; 2],
    /// Zero-sized tag type for compile-time type safety.
    _phantom: PhantomData<T>,
}

impl<T> fmt::Display for BmsChannelId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "guarded by match on bytes.len() or bytes[1] sentinel check"
)]
impl<T> BmsChannelId<T> {
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
}

#[expect(
    clippy::indexing_slicing,
    reason = "guarded by match on bytes.len() or sentinel check"
)]
impl<T> TryFrom<&str> for BmsChannelId<T> {
    type Error = BmsChannelIdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        match bytes.len() {
            1 if is_bms_alphanumeric(bytes[0]) => Ok(Self {
                bytes: [bytes[0], 0],
                _phantom: PhantomData,
            }),
            2 if is_bms_alphanumeric(bytes[0]) && is_bms_alphanumeric(bytes[1]) => Ok(Self {
                bytes: [bytes[0], bytes[1]],
                _phantom: PhantomData,
            }),
            _ => Err(BmsChannelIdError {
                input: s.to_owned(),
            }),
        }
    }
}

/// Allows `value.parse::<BmsChannelId<T>>()` with unified error conversion.
impl<T> FromStr for BmsChannelId<T> {
    type Err = BmsChannelIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

/// Error returned when a channel ID string is not a valid 1–2 character
/// `0`–`9` `A`–`Z` `a`–`z` sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsChannelIdError {
    /// The raw string that failed validation.
    pub input: String,
}

impl fmt::Display for BmsChannelIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid channel id: {:?}", self.input)
    }
}

impl std::error::Error for BmsChannelIdError {}

impl From<BmsChannelIdError> for BmsTokenizeError {
    fn from(e: BmsChannelIdError) -> Self {
        BmsTokenizeError::InvalidChannelId(e.input)
    }
}

// Tag types — zero-sized, never instantiated.

/// Tag type for `#WAVxx` / `#EXWAVxx` index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavTag {}
/// Tag type for `#BMPxx` / `#EXBMPxx` / `#BGAxx` / etc. index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmpTag {}
/// Tag type for `#BPMxx` index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpmTag {}
/// Tag type for `#STOPxx` index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTag {}
/// Tag type for `#SCROLLxx` index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollTag {}
/// Tag type for `#SPEEDxx` index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedTag {}
/// Tag type for `#EXRANKxx` index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExRankTag {}
/// Tag type for `#SEEKxx` index IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekTag {}
/// Tag type for `#LNOBJ` value (a WAV index used as LN terminator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LnObjTag {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_two_char_ids() {
        assert!(BmsChannelId::<WavTag>::try_from("01").is_ok());
        assert!(BmsChannelId::<WavTag>::try_from("ZZ").is_ok());
        assert!(BmsChannelId::<WavTag>::try_from("aa").is_ok());
        assert!(BmsChannelId::<WavTag>::try_from("zZ").is_ok());
        assert!(BmsChannelId::<WavTag>::try_from("FF").is_ok());
    }

    #[test]
    fn valid_one_char_ids() {
        assert!(BmsChannelId::<WavTag>::try_from("A").is_ok());
        assert!(BmsChannelId::<WavTag>::try_from("9").is_ok());
        assert!(BmsChannelId::<WavTag>::try_from("0").is_ok());
    }

    #[test]
    fn empty_id_is_invalid() {
        assert!(BmsChannelId::<WavTag>::try_from("").is_err());
    }

    #[test]
    fn three_char_id_is_invalid() {
        assert!(BmsChannelId::<WavTag>::try_from("AAA").is_err());
    }

    #[test]
    fn non_alphanumeric_is_invalid() {
        assert!(BmsChannelId::<WavTag>::try_from("*!").is_err());
        assert!(BmsChannelId::<WavTag>::try_from(" ").is_err());
        assert!(BmsChannelId::<WavTag>::try_from("ab:").is_err());
        assert!(BmsChannelId::<WavTag>::try_from("-1").is_err());
    }

    #[test]
    fn different_tags_prevent_mixing() {
        let wav_id = BmsChannelId::<WavTag>::try_from("01").unwrap();
        let bmp_id = BmsChannelId::<BmpTag>::try_from("01").unwrap();
        assert_eq!(wav_id.as_str(), bmp_id.as_str());
    }

    #[test]
    fn as_str_returns_original() {
        let id = BmsChannelId::<WavTag>::try_from("2A").unwrap();
        assert_eq!(id.as_str(), "2A");
    }

    #[test]
    fn as_str_one_char() {
        let id = BmsChannelId::<WavTag>::try_from("A").unwrap();
        assert_eq!(id.as_str(), "A");
    }

    #[test]
    fn as_bytes_two_char() {
        let id = BmsChannelId::<WavTag>::try_from("2A").unwrap();
        assert_eq!(id.as_bytes(), &[b'2', b'A']);
    }

    #[test]
    fn as_bytes_one_char() {
        let id = BmsChannelId::<WavTag>::try_from("A").unwrap();
        assert_eq!(id.as_bytes(), &[b'A']);
    }

    #[test]
    fn display_output() {
        let id = BmsChannelId::<WavTag>::try_from("FF").unwrap();
        assert_eq!(id.to_string(), "FF");
    }

    #[test]
    fn bms_channel_id_error_display() {
        let err = BmsChannelIdError {
            input: "!!!".to_string(),
        };
        assert!(err.to_string().contains("!!!"));
    }

    #[test]
    fn tag_types_are_zero_sized() {
        assert_eq!(std::mem::size_of::<WavTag>(), 0);
        assert_eq!(std::mem::size_of::<BmpTag>(), 0);
        assert_eq!(std::mem::size_of::<BpmTag>(), 0);
    }

    #[test]
    fn size_of_bms_channel_id() {
        // [u8; 2] + PhantomData<T> → 2 bytes, no pointer overhead.
        assert_eq!(std::mem::size_of::<BmsChannelId<WavTag>>(), 2);
    }
}
