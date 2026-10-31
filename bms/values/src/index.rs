//! Type-safe BMS index wrapper types.
//!
//! BMS uses 1-2 character indices to reference resources (WAV, BMP), timing
//! definitions (BPM, STOP, SCROLL, SPEED), channel numbers, and other indexed
//! commands.
//!
//! [`BmsIndex`] is the raw storage type with Base62 character validation. Each
//! semantic index is a newtype wrapper around `BmsIndex` (for example,
//! [`WavIndex`], [`BpmIndex`]), providing compile-time type safety.
//!
//! # Character sets
//!
//! Most indices accept Base62 characters (`0-9A-Za-z`). [`ChannelIndex`] is
//! special: it validates Base36 (`0-9A-Z`) because channel numbers in BMS
//! message lines use hexadecimal.
//!
//! The [`BmsBase`] enum and [`BmsIndex::is_valid_for`] support runtime
//! character-set validation where needed.
//!
//! # Storage
//!
//! IDs are stored as `[u8; 2]`, one raw ASCII character per byte. For a
//! single-character ID (such as `"A"`), the second byte is `0`.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use derive_more::Display;
use thiserror::Error;

use crate::{BmsIndexNewtype, BmsTokenAttr, IntoTokensError};

// Character-set validation helpers

/// Base-62 characters: `0`-`9`, `A`-`Z`, `a`-`z`.
#[inline]
#[must_use]
#[expect(
    clippy::redundant_pub_crate,
    reason = "needed for sibling module access"
)]
pub(crate) const fn is_base62(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
}

// BmsBase: runtime character-set enum

/// Character set used to validate BMS indices.
///
/// It replaces the former compile-time `BmsCharset` type parameter. Use
/// [`BmsIndex::is_valid_for`] for runtime validation.
///
/// [`Default`] is [`Base36`](Self::Base36), the default character set for
/// standard BMS files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr, Default)]
#[non_exhaustive]
pub enum BmsBase {
    /// Hexadecimal (`0`-`9`, `A`-`F`, `a`-`f`; 16 values per position).
    #[bms_token("16")]
    Base16,
    /// Base-36 uppercase (`0`-`9`, `A`-`Z`; 36 values per position).
    ///
    /// The default character set for standard and most common BMS files, hence
    /// the [`Default`].
    #[default]
    #[bms_token("36")]
    Base36,
    /// Base-62 (`0`-`9`, `A`-`Z`, `a`-`z`; 62 values per position).
    /// This is the default character set for most BMS indices.
    #[bms_token("62")]
    Base62,
}

impl BmsBase {
    /// Check whether byte `b` is valid in this base's character set.
    #[must_use]
    pub const fn is_valid_char(self, b: u8) -> bool {
        match self {
            Self::Base16 => b.is_ascii_hexdigit(),
            Self::Base36 => matches!(b, b'0'..=b'9' | b'A'..=b'Z'),
            Self::Base62 => matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'),
        }
    }

    /// Decode a two-character string into a numeric value in this base (Base36
    /// decoding).
    ///
    /// Base36 and Base62 use the same digit mapping; for Base16 use
    /// [`BmsIndex::as_u8_hex`]. The input must be exactly 2 characters and
    /// every character must be valid.
    #[must_use]
    pub fn decode(self, s: &str) -> Option<u16> {
        match s.as_bytes() {
            [b1, b2] => {
                let hi = base36_digit_value(*b1)?;
                let lo = base36_digit_value(*b2)?;
                Some(hi * 36 + lo)
            }
            _ => None,
        }
    }
}

// BmsIndex: raw storage (no generics)

/// A validated 1-2 character BMS index stored as raw ASCII bytes.
///
/// Construction through [`TryFrom<&str>`] / [`FromStr`] validates with the
/// most permissive Base62 character set (`0-9A-Za-z`). See [`BmsBase`] and
/// [`is_valid_for`](Self::is_valid_for) for runtime character-set validation.
///
/// # Storage
///
/// `[u8; 2]`: when the ID is a single character, `bytes[1]` is `0`.
#[derive(Clone, Copy, Debug)]
pub struct BmsIndex {
    /// Raw ASCII bytes of the characters.
    ///
    /// # Invariants
    ///
    /// When the ID is a single character (such as `"A"`), `bytes[1]` is `0`.
    /// A two-character ID stores both bytes; neither is `0` because every
    /// character-set validator excludes `NUL`.
    bytes: [u8; 2],
}

// Custom comparison traits: case-sensitive.
//
// Standard BMS (Base36) normalizes indices to uppercase in the parser
// (`BmsIndex::normalize(BmsBase::Base36)`), so every stored key is uppercase.
// Base62 mode preserves the original case. Because every insertion and lookup
// path normalizes consistently, comparisons here are always **case-sensitive**,
// which is required for `#BASE 62` (`"aa"` and `"AA"` must be distinct keys).

impl PartialEq for BmsIndex {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for BmsIndex {}

impl PartialOrd for BmsIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BmsIndex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl Hash for BmsIndex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "guarded by match on bytes.len() or bytes[1] sentinel check"
)]
impl BmsIndex {
    /// Return the ASCII string representation of this ID (borrowed from self).
    ///
    /// # Panics
    ///
    /// Never panics in practice: bytes are validated as ASCII at construction,
    /// so the UTF-8 conversion cannot fail.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "bytes are ASCII by the BmsIndex construction invariant"
    )]
    pub fn as_str(&self) -> &str {
        let len = if self.bytes[1] == 0 { 1 } else { 2 };
        std::str::from_utf8(&self.bytes[..len]).expect("bytes validated ASCII on construction")
    }

    /// Return the raw ASCII bytes (1 or 2 bytes).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        let len = if self.bytes[1] == 0 { 1 } else { 2 };
        &self.bytes[..len]
    }

    /// Convert Base-36 characters to a numeric index.
    ///
    /// `"00"` gives `0` and `"ZZ"` gives `1295`. A single-character ID such as
    /// `"A"` returns `10` (without applying the positional multiplier).
    ///
    /// Returns `None` when a character cannot be decoded (which should not
    /// happen for IDs constructed through [`TryFrom`] / [`FromStr`]).
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

    /// Convert hexadecimal characters to a `u8` value.
    ///
    /// A two-character ID such as `"0A"` returns `10`. A single-character ID
    /// such as `"A"` returns `10` (without shifting).
    ///
    /// Returns `None` when either byte is not a valid hexadecimal character.
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

    /// Check whether every character in this index is valid for the given
    /// character set.
    #[must_use]
    pub const fn is_valid_for(&self, base: BmsBase) -> bool {
        base.is_valid_char(self.bytes[0])
            && (self.bytes[1] == 0 || base.is_valid_char(self.bytes[1]))
    }

    /// Create from validated bytes (for use inside newtypes).
    #[inline]
    #[must_use]
    const fn from_valid(bytes: [u8; 2]) -> Self {
        Self { bytes }
    }

    /// Return a copy normalized for the given base.
    ///
    /// For [`BmsBase::Base16`] and [`BmsBase::Base36`] (standard BMS character
    /// sets), this uppercases the bytes so `"AA"` and `"aa"` map to the same
    /// index. For [`BmsBase::Base62`], the original case is preserved
    /// (case-sensitive).
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "to_ascii_uppercase is not const"
    )]
    pub fn normalize(&self, base: BmsBase) -> Self {
        match base {
            BmsBase::Base16 | BmsBase::Base36 => Self {
                bytes: [
                    self.bytes[0].to_ascii_uppercase(),
                    self.bytes[1].to_ascii_uppercase(),
                ],
            },
            BmsBase::Base62 => *self,
        }
    }
}

impl fmt::Display for BmsIndex {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for BmsIndex {
    type Error = BmsIndexError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.as_bytes() {
            [b] if is_base62(*b) => Ok(Self { bytes: [*b, 0] }),
            [a, b] if is_base62(*a) && is_base62(*b) => Ok(Self { bytes: [*a, *b] }),
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

/// Decode a single hexadecimal ASCII byte into a value (0-15).
const fn hex_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Decode a single Base-36 ASCII byte into a value (0-35).
#[must_use]
pub const fn base36_digit_value(b: u8) -> Option<u16> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u16),
        b'A'..=b'Z' => Some((b - b'A') as u16 + 10),
        b'a'..=b'z' => Some((b - b'a') as u16 + 10),
        _ => None,
    }
}

// Error types

/// Error returned when a BMS index string is not a valid 1-2 character
/// sequence in the required character set.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid BMS index: {input}")]
pub struct BmsIndexError {
    /// Original string that failed validation.
    pub input: String,
}

impl IntoTokensError for BmsIndexError {
    #[inline]
    fn into_error(self, _context: &'static str, value: String) -> crate::BmsTokenizeError {
        crate::BmsTokenizeError::InvalidInteger { value }
    }
}

// Newtype wrappers, one per semantic index type

// Each newtype:
//   - wraps BmsIndex
//   - provides Deref<Target = BmsIndex> for transparent method access
//   - validates the required character set at construction
//   - implements Display, FromStr, TryFrom<&str>, Clone, Copy, and so on

// Standard Base62 newtypes

/// Index for `#WAV{id}` / `#EXWAV{id}`: audio definition reference.
#[derive(BmsIndexNewtype)]
pub struct WavIndex(pub BmsIndex);

/// Index for `#BMP{id}` / `#BGA{id}` / `#@BGA{id}` / `#SWBGA{id}` /
/// `#ARGB{id}` / `#EXBMP{id}`: image and BGA definition references.
#[derive(BmsIndexNewtype)]
pub struct BmpIndex(pub BmsIndex);

/// Index for `#BPM{id}` / `#EXBPM{id}`: extended BPM definition reference.
#[derive(BmsIndexNewtype)]
pub struct BpmIndex(pub BmsIndex);

/// Index for `#STOP{id}`: stop definition reference.
#[derive(BmsIndexNewtype)]
pub struct StopIndex(pub BmsIndex);

/// Index for `#SCROLL{id}`: scroll speed definition reference.
#[derive(BmsIndexNewtype)]
pub struct ScrollIndex(pub BmsIndex);

/// Index for `#SPEED{id}`: speed and spacing definition reference.
#[derive(BmsIndexNewtype)]
pub struct SpeedIndex(pub BmsIndex);

/// Index for `#EXRANK{id}`: per-position judge override reference.
#[derive(BmsIndexNewtype)]
pub struct ExRankIndex(pub BmsIndex);

/// Index for `#SEEK{id}`: video seek position reference.
#[derive(BmsIndexNewtype)]
pub struct SeekIndex(pub BmsIndex);

/// Index for `#LNOBJ`: the WAV index used as a long-note terminator marker.
#[derive(BmsIndexNewtype)]
pub struct LnObjIndex(pub BmsIndex);

/// Index for `#TEXT{id}` / `#SONG{id}`: timed on-screen text reference.
#[derive(BmsIndexNewtype)]
pub struct TextIndex(pub BmsIndex);

/// Index for `#CHANGEOPTION{id}`: dynamic option change reference.
#[derive(BmsIndexNewtype)]
pub struct ChangeOptionIndex(pub BmsIndex);

/// Index for a 2-character object ID in a message body value.
///
/// This is the object index parsed leniently from the body string of a
/// [`BmsMessage`](crate::BmsMessage): every pair of consecutive valid Base62
/// characters forms one object ID.
#[derive(BmsIndexNewtype)]
pub struct ObjectIndex(pub BmsIndex);

// Special newtype: ChannelIndex (Base36 validation)

/// A channel number in a BMS message line (`#xxxYY:values`).
///
/// Unlike the default Base62 validation, it validates Base36 (`0-9A-Z`).
/// Use [`as_u8_hex`](BmsIndex::as_u8_hex) to convert to a byte value.
///
/// # Construction
///
/// Parse through [`FromStr`] (which validates Base36), or build directly from
/// known-valid bytes with [`ChannelIndex::from_valid`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display("{}", _0)]
pub struct ChannelIndex(BmsIndex);

impl ChannelIndex {
    /// Construct from a known-valid Base36 byte pair (no validation).
    ///
    /// The caller must guarantee that every character in `bytes` is in
    /// `0-9A-Z`.
    #[inline]
    #[must_use]
    pub const fn from_valid(bytes: [u8; 2]) -> Self {
        Self(BmsIndex::from_valid(bytes))
    }

    /// Return the byte representation of the underlying index.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Interpret `ChannelIndex` as a byte value in uppercase hexadecimal.
    /// Returns `None` when either character is outside `0-9A-F`.
    #[inline]
    #[must_use]
    pub fn as_u8_hex(&self) -> Option<u8> {
        self.0.as_u8_hex()
    }

    /// Return a reference to the underlying index.
    #[inline]
    #[must_use]
    pub const fn as_bms_index(&self) -> &BmsIndex {
        &self.0
    }

    /// Convert into the underlying index.
    #[inline]
    #[must_use]
    pub const fn into_bms_index(self) -> BmsIndex {
        self.0
    }
}

impl FromStr for ChannelIndex {
    type Err = BmsIndexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.as_bytes() {
            [b] if BmsBase::Base36.is_valid_char(*b) => Ok(Self(BmsIndex::from_valid([*b, 0]))),
            [a, b] if BmsBase::Base36.is_valid_char(*a) && BmsBase::Base36.is_valid_char(*b) => {
                Ok(Self(BmsIndex::from_valid([*a, *b])))
            }
            _ => Err(BmsIndexError {
                input: s.to_owned(),
            }),
        }
    }
}

impl TryFrom<&str> for ChannelIndex {
    type Error = BmsIndexError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

// Tests
