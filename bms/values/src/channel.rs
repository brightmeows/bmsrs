//! Typed BMS channel enum.
//!
//! Classifies channel numbers in BMS message lines (`#xxxYY:values`) into known
//! semantic variants, note channels, and unknown or reserved channels.

use std::fmt;

use crate::index::ChannelIndex;

/// A classified BMS channel identifier.
///
/// Every known non-note channel is a unit variant. Note channels carry their
/// raw [`ChannelIndex`] for downstream parsing (player side, type, and key
/// number are the parser's concern). Unknown channels also carry the raw index
/// so custom and engine-specific channels survive in the token stream.
///
/// The inner [`ChannelIndex`] validates Base36 (`0-9A-Z`); construction
/// normalizes input to uppercase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BmsChannel {
    // Known non-note channels (hexadecimal, 01-0E)
    /// Channel `01`: BGM.
    Bgm,
    /// Channel `02`: measure length.
    MeasureLength,
    /// Channel `03`: BPM change (hexadecimal integer `01`-`FF`).
    BpmChange,
    /// Channel `04`: BGA BASE layer.
    BgaBase,
    /// Channel `05`: extended object / SEEK.
    Seek,
    /// Channel `06`: BGA POOR (miss) layer.
    BgaPoor,
    /// Channel `07`: BGA LAYER overlay.
    BgaLayer,
    /// Channel `08`: extended BPM (references `#BPMxx` / `#EXBPMxx`).
    ExtendedBpm,
    /// Channel `09`: STOP sequence (references `#STOPxx`).
    Stop,
    /// Channel `0A`: BGA LAYER2.
    BgaLayer2,
    /// Channel `0B`: BGA BASE opacity.
    BgaBaseOpacity,
    /// Channel `0C`: BGA LAYER opacity.
    BgaLayerOpacity,
    /// Channel `0D`: BGA LAYER2 opacity.
    BgaLayer2Opacity,
    /// Channel `0E`: BGA POOR opacity.
    BgaPoorOpacity,

    // Known non-note channels (hexadecimal, 97-A6)
    /// Channel `97`: BGM volume.
    BgmVolume,
    /// Channel `98`: KEY volume.
    KeyVolume,
    /// Channel `99`: TEXT (references `#TEXTxx`).
    Text,
    /// Channel `A0`: JUDGE / EXRANK.
    Judge,
    /// Channel `A1`: BGA BASE aRGB.
    BgaArgbBase,
    /// Channel `A2`: BGA LAYER aRGB.
    BgaArgbLayer,
    /// Channel `A3`: BGA LAYER2 aRGB.
    BgaArgbLayer2,
    /// Channel `A4`: BGA POOR aRGB.
    BgaArgbPoor,
    /// Channel `A5`: BGA KEYBOUND.
    BgaKeyBound,
    /// Channel `A6`: OPTION.
    Option,

    // Extended non-note channels (non-hexadecimal)
    /// Channel `SC`: SCROLL (scroll speed multiplier).
    Scroll,
    /// Channel `SP`: SPEED (visual note spacing).
    Speed,

    // Note channels
    /// A note channel in one of the known playable ranges:
    /// `11-1Z`, `21-2Z`, `31-3Z`, `41-4Z`, `51-5Z`, `61-6Z`,
    /// `D1-D9`, `E1-E9`.
    ///
    /// The raw [`ChannelIndex`] carries the full channel string so the parser
    /// can extract the player side, note type, and key number.
    Note(ChannelIndex),

    // Unknown / reserved
    /// Unknown or reserved channel (such as `00`, `0F`, `10`, `20`, `70-96`,
    /// `ZZ`, and so on).
    Unknown(ChannelIndex),
}

// Construction

impl BmsChannel {
    /// Create a `BmsChannel` from a raw channel string.
    ///
    /// The input is normalized to uppercase before parsing. Returns `None` when
    /// the string is not a valid 1-2 character Base36 identifier.
    #[must_use]
    pub fn from_raw(s: &str) -> Option<Self> {
        let upper = s.to_ascii_uppercase();
        let idx: ChannelIndex = upper.as_str().try_into().ok()?;
        Some(classify_channel(idx))
    }

    /// Return the channel number string (uppercase, 1-2 characters).
    ///
    /// Known non-note channels return their canonical number (such as `"01"`
    /// or `"SC"`); variants carrying a raw index return the uppercase form of
    /// that index. Used by line-level editing when assembling the address
    /// segment.
    #[must_use]
    pub fn as_str(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Note(idx) | Self::Unknown(idx) => std::borrow::Cow::Owned(idx.to_string()),
            fixed => std::borrow::Cow::Borrowed(fixed_number(*fixed)),
        }
    }
}

/// Canonical number string for a known non-note channel.
const fn fixed_number(ch: BmsChannel) -> &'static str {
    match ch {
        BmsChannel::Bgm => "01",
        BmsChannel::MeasureLength => "02",
        BmsChannel::BpmChange => "03",
        BmsChannel::BgaBase => "04",
        BmsChannel::Seek => "05",
        BmsChannel::BgaPoor => "06",
        BmsChannel::BgaLayer => "07",
        BmsChannel::ExtendedBpm => "08",
        BmsChannel::Stop => "09",
        BmsChannel::BgaLayer2 => "0A",
        BmsChannel::BgaBaseOpacity => "0B",
        BmsChannel::BgaLayerOpacity => "0C",
        BmsChannel::BgaLayer2Opacity => "0D",
        BmsChannel::BgaPoorOpacity => "0E",
        BmsChannel::BgmVolume => "97",
        BmsChannel::KeyVolume => "98",
        BmsChannel::Text => "99",
        BmsChannel::Judge => "A0",
        BmsChannel::BgaArgbBase => "A1",
        BmsChannel::BgaArgbLayer => "A2",
        BmsChannel::BgaArgbLayer2 => "A3",
        BmsChannel::BgaArgbPoor => "A4",
        BmsChannel::BgaKeyBound => "A5",
        BmsChannel::Option => "A6",
        BmsChannel::Scroll => "SC",
        BmsChannel::Speed => "SP",
        // Variants carrying an index are handled in `as_str`.
        BmsChannel::Note(_) | BmsChannel::Unknown(_) => "",
    }
}

/// Classify a validated [`ChannelIndex`] into a [`BmsChannel`] variant.
///
/// Every valid `ChannelIndex` maps to exactly one variant; there is no failing
/// path.
///
/// The input index should contain uppercase characters (the caller should
/// normalize before constructing the index).
#[must_use]
#[expect(
    clippy::unreachable,
    reason = "match arms confirmed by bytes.len() check above"
)]
pub fn classify_channel(ch: ChannelIndex) -> BmsChannel {
    let bytes = ch.as_bytes();

    match bytes.len() {
        1 => {
            // Single-character channel: one hexadecimal digit 0-F.
            // Input is already uppercase (normalized by the caller).
            let &[b] = bytes else {
                unreachable!("bytes.len() == 1 confirmed by match")
            };
            match b {
                b'1' => BmsChannel::Bgm,
                b'2' => BmsChannel::MeasureLength,
                b'3' => BmsChannel::BpmChange,
                b'4' => BmsChannel::BgaBase,
                b'5' => BmsChannel::Seek,
                b'6' => BmsChannel::BgaPoor,
                b'7' => BmsChannel::BgaLayer,
                b'8' => BmsChannel::ExtendedBpm,
                b'9' => BmsChannel::Stop,
                b'A' => BmsChannel::BgaLayer2,
                b'B' => BmsChannel::BgaBaseOpacity,
                b'C' => BmsChannel::BgaLayerOpacity,
                b'D' => BmsChannel::BgaLayer2Opacity,
                b'E' => BmsChannel::BgaPoorOpacity,
                // '0' (reserved), 'F' (no defined channel), and any other
                // value fall into Unknown.
                _ => BmsChannel::Unknown(ch),
            }
        }
        2 => {
            // Input is already uppercase (normalized by the caller).
            let &[first, second] = bytes else {
                unreachable!("bytes.len() == 2 confirmed by match")
            };

            // Exact matches for known channels.
            match (first, second) {
                (b'0', b'1') => return BmsChannel::Bgm,
                (b'0', b'2') => return BmsChannel::MeasureLength,
                (b'0', b'3') => return BmsChannel::BpmChange,
                (b'0', b'4') => return BmsChannel::BgaBase,
                (b'0', b'5') => return BmsChannel::Seek,
                (b'0', b'6') => return BmsChannel::BgaPoor,
                (b'0', b'7') => return BmsChannel::BgaLayer,
                (b'0', b'8') => return BmsChannel::ExtendedBpm,
                (b'0', b'9') => return BmsChannel::Stop,
                (b'0', b'A') => return BmsChannel::BgaLayer2,
                (b'0', b'B') => return BmsChannel::BgaBaseOpacity,
                (b'0', b'C') => return BmsChannel::BgaLayerOpacity,
                (b'0', b'D') => return BmsChannel::BgaLayer2Opacity,
                (b'0', b'E') => return BmsChannel::BgaPoorOpacity,
                (b'9', b'7') => return BmsChannel::BgmVolume,
                (b'9', b'8') => return BmsChannel::KeyVolume,
                (b'9', b'9') => return BmsChannel::Text,
                (b'A', b'0') => return BmsChannel::Judge,
                (b'A', b'1') => return BmsChannel::BgaArgbBase,
                (b'A', b'2') => return BmsChannel::BgaArgbLayer,
                (b'A', b'3') => return BmsChannel::BgaArgbLayer2,
                (b'A', b'4') => return BmsChannel::BgaArgbPoor,
                (b'A', b'5') => return BmsChannel::BgaKeyBound,
                (b'A', b'6') => return BmsChannel::Option,
                (b'S', b'C') => return BmsChannel::Scroll,
                (b'S', b'P') => return BmsChannel::Speed,
                _ => {}
            }

            // Note channel detection.
            //
            // Ranges:
            //   '1'..='6' + second != '0' means playable / long note
            //   'D'/'E'   + '1'..='9'     means mine
            if first.is_ascii_digit() && (b'1'..=b'6').contains(&first) && second != b'0' {
                return BmsChannel::Note(ch);
            }
            if matches!(first, b'D' | b'E')
                && second.is_ascii_digit()
                && (b'1'..=b'9').contains(&second)
            {
                return BmsChannel::Note(ch);
            }

            BmsChannel::Unknown(ch)
        }
        _ => {
            // `as_bytes()` guarantees 1 or 2 bytes; this branch is unreachable.
            unreachable!("ChannelIndex::as_bytes() returns 1 or 2 bytes")
        }
    }
}

// Accessors

impl BmsChannel {
    /// Return the hexadecimal `u8` value of this channel, when applicable.
    ///
    /// Unit variants return their canonical hexadecimal value (for example,
    /// [`BmsChannel::Bgm`] gives `Some(0x01)`). Non-hexadecimal channels
    /// ([`Scroll`](Self::Scroll), [`Speed`](Self::Speed)) return `None`.
    /// [`Note`](Self::Note) and [`Unknown`](Self::Unknown) delegate to
    /// [`crate::index::BmsIndex::as_u8_hex`].
    #[must_use]
    pub fn as_u8_hex(&self) -> Option<u8> {
        match self {
            Self::Bgm => Some(0x01),
            Self::MeasureLength => Some(0x02),
            Self::BpmChange => Some(0x03),
            Self::BgaBase => Some(0x04),
            Self::Seek => Some(0x05),
            Self::BgaPoor => Some(0x06),
            Self::BgaLayer => Some(0x07),
            Self::ExtendedBpm => Some(0x08),
            Self::Stop => Some(0x09),
            Self::BgaLayer2 => Some(0x0A),
            Self::BgaBaseOpacity => Some(0x0B),
            Self::BgaLayerOpacity => Some(0x0C),
            Self::BgaLayer2Opacity => Some(0x0D),
            Self::BgaPoorOpacity => Some(0x0E),
            Self::BgmVolume => Some(0x97),
            Self::KeyVolume => Some(0x98),
            Self::Text => Some(0x99),
            Self::Judge => Some(0xA0),
            Self::BgaArgbBase => Some(0xA1),
            Self::BgaArgbLayer => Some(0xA2),
            Self::BgaArgbLayer2 => Some(0xA3),
            Self::BgaArgbPoor => Some(0xA4),
            Self::BgaKeyBound => Some(0xA5),
            Self::Option => Some(0xA6),
            Self::Scroll | Self::Speed => None,
            Self::Note(raw) | Self::Unknown(raw) => raw.as_u8_hex(),
        }
    }
}

// Display

impl fmt::Display for BmsChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bgm => f.write_str("01"),
            Self::MeasureLength => f.write_str("02"),
            Self::BpmChange => f.write_str("03"),
            Self::BgaBase => f.write_str("04"),
            Self::Seek => f.write_str("05"),
            Self::BgaPoor => f.write_str("06"),
            Self::BgaLayer => f.write_str("07"),
            Self::ExtendedBpm => f.write_str("08"),
            Self::Stop => f.write_str("09"),
            Self::BgaLayer2 => f.write_str("0A"),
            Self::BgaBaseOpacity => f.write_str("0B"),
            Self::BgaLayerOpacity => f.write_str("0C"),
            Self::BgaLayer2Opacity => f.write_str("0D"),
            Self::BgaPoorOpacity => f.write_str("0E"),
            Self::BgmVolume => f.write_str("97"),
            Self::KeyVolume => f.write_str("98"),
            Self::Text => f.write_str("99"),
            Self::Judge => f.write_str("A0"),
            Self::BgaArgbBase => f.write_str("A1"),
            Self::BgaArgbLayer => f.write_str("A2"),
            Self::BgaArgbLayer2 => f.write_str("A3"),
            Self::BgaArgbPoor => f.write_str("A4"),
            Self::BgaKeyBound => f.write_str("A5"),
            Self::Option => f.write_str("A6"),
            Self::Scroll => f.write_str("SC"),
            Self::Speed => f.write_str("SP"),
            Self::Note(raw) | Self::Unknown(raw) => fmt::Display::fmt(raw, f),
        }
    }
}
