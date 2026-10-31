//! BMS message (channel data) line parsing.
//!
//! # Format
//!
//! Channel data lines have the form `#ADDR:body`, where:
//!
//! - `ADDR` is a string made of a **measure** (decimal digits, zero-based)
//!   followed by a **channel** (the last one or two valid Base62 characters,
//!   `0-9A-Za-z`). For example, in `#00111:...`, address `00111` has measure
//!   `001` and channel `11`.
//! - `body` is the raw value string. Two-character object indices are parsed
//!   downstream (by the parser) from the concatenated raw storage. Unknown or
//!   extended channels (such as `SC`, `SP`, `1G`) are kept in the
//!   [`channel`](BmsMessage::channel) field and in the raw storage.
//!
//! # Channel semantics (excerpt)
//!
//! The full channel map is the parser's responsibility; this table only lists
//! the most common channels for reference:
//!
//! | Channel | Purpose |
//! |---------|---------|
//! | `01` | BGM (may span multiple lines) |
//! | `02` | Measure length change |
//! | `03` | BPM change (hexadecimal integer, `[01-FF]`) |
//! | `04` | BGA BASE layer |
//! | `06` | BGA POOR (miss) layer |
//! | `07` | BGA LAYER (black means transparent) |
//! | `08` | Extended BPM change |
//! | `09` | STOP sequence |
//! | `0A` | BGA LAYER2 (SCROLL) |
//! | `11-19` | 1P visible notes |
//! | `21-29` | 2P visible notes |
//! | `31-39` | 1P invisible notes |
//! | `41-49` | 2P invisible notes |
//! | `51-69` | Long-note channels |
//! | `D1-D9` | 1P mines |
//! | `E1-E9` | 2P mines |
//! | `SC` | SCROLL (extension) |
//! | `SP` | SPEED (extension) |
//!
//! # Example
//!
//! ```text
//! #00111:11223344  addr="00111", body="11223344"
//!                  track=1, channel="11"
//! #0010A:01        addr="0010A", body="01"
//!                  track=1, channel="0A"
//! #000SC:          addr="000SC", body=""
//!                  track=0, channel="SC"
//! ```

use crate::channel::{BmsChannel, classify_channel};
use crate::index::{ChannelIndex, is_base62};
use crate::{BmsToken, BmsTokenizeError, BmsTryFromError};

/// A channel data line in a BMS file (`#ADDR:body`).
///
/// See the module-level documentation for the format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsMessage {
    /// Raw address string (before `:`).
    pub addr: String,
    /// Raw body string (after `:`).
    pub body: String,

    /// Zero-based measure number, extracted from the decimal digits in
    /// [`addr`](BmsMessage::addr) before the channel suffix.
    track: u16,
    /// Channel number: the last one or two valid Base62 characters
    /// (`0-9A-Za-z`) in [`addr`](BmsMessage::addr), classified into the
    /// [`BmsChannel`] enum.
    channel: BmsChannel,
}

impl BmsMessage {
    /// Return the zero-based measure number.
    #[inline]
    #[must_use]
    pub const fn track(&self) -> u16 {
        self.track
    }

    /// Return the parsed channel number.
    #[inline]
    #[must_use]
    pub const fn channel(&self) -> BmsChannel {
        self.channel
    }

    /// Construct a message line from its parts.
    ///
    /// Line-level editors use this to rebuild a [`BmsMessage`] when repacking
    /// channel data, forming the unpack/pack inverse of
    /// [`parse_message_line`](crate::parse_message_line).
    ///
    /// `channel` must be a 1-2 character Base62 identifier; lowercase input is
    /// normalized to uppercase at construction, matching parse behavior.
    ///
    /// # Errors
    ///
    /// Returns [`BmsTokenizeError::InvalidChannel`] when `channel` is not a
    /// valid Base62 channel number.
    pub fn compose(
        track: u16,
        channel: &str,
        body: impl Into<String>,
    ) -> Result<Self, BmsTokenizeError> {
        let upper = channel.to_ascii_uppercase();
        let idx: ChannelIndex =
            upper
                .as_str()
                .try_into()
                .map_err(|_e| BmsTokenizeError::InvalidChannel {
                    value: channel.to_owned(),
                })?;
        Ok(Self {
            addr: format!("{track:03}{upper}"),
            body: body.into(),
            track,
            channel: classify_channel(idx),
        })
    }

    /// Format as a single line of text (`#ADDR:body`).
    ///
    /// The address is emitted as a 3-digit zero-padded measure followed by the
    /// normalized (uppercase) channel number; `body` is emitted verbatim. Used
    /// by line-level editing when writing back.
    #[must_use]
    pub fn to_line_text(&self) -> String {
        format!("#{}:{}", self.addr, self.body)
    }

    /// Pack a sequence of 2-character object indices into a channel body
    /// string.
    ///
    /// Packing is concatenation in order; each value must be exactly 2
    /// characters, and the caller is responsible for keeping the slot count
    /// consistent with the resolution.
    #[must_use]
    pub fn encode_body<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
        values.into_iter().collect()
    }
}

impl TryFrom<BmsToken> for BmsMessage {
    type Error = BmsTryFromError;

    #[inline]
    fn try_from(token: BmsToken) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Message(m) => Ok(m),
            BmsToken::Header(_) => Err(BmsTryFromError::NotAMessage),
        }
    }
}

/// Try to parse a single line as a BMS channel message.
///
/// Returns `Ok(None)` when the line does not look like a channel message (for
/// example, a header, comment, or blank line). Returns `Err(...)` when the line
/// looks like a channel message but has no valid channel suffix.
///
/// # Errors
///
/// Returns [`BmsTokenizeError::InvalidChannel`] when the address contains a
/// non-Base62 character at the channel position, or the channel suffix is
/// unrecognized.
#[expect(
    clippy::string_slice,
    reason = "BMS message lines are ASCII-only (hex digits, Base62 chars, colons); byte indexing is safe"
)]
pub fn parse_message_line(line: &str) -> Result<Option<BmsMessage>, BmsTokenizeError> {
    if line.is_empty() || !line.starts_with('#') {
        return Ok(None);
    }

    // Needs at least: `#` + 1 character + `:` (for example, `"#1:"`).
    if line.len() < 3 {
        return Ok(None);
    }

    let rest = &line[1..]; // Strip the leading `#`.

    // Split at `:` into addr and body.
    let Some((addr, body)) = rest.split_once(':') else {
        return Ok(None); // No colon: not a message line.
    };

    if addr.is_empty() || !addr.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        // addr must start with a digit (the measure prefix) to count as a
        // message line; otherwise this is a header that happens to contain a
        // colon, such as `#SWBGA01 30:...`.
        return Ok(None);
    }

    // Parse the channel from the end of addr:
    // take the last one or two consecutive valid Base62 characters.
    #[expect(
        clippy::indexing_slicing,
        reason = "addr is non-empty (checked above); `len` guards indices"
    )]
    let (channel_str, prefix) = {
        let bytes = addr.as_bytes();
        let len = bytes.len();
        let last = bytes[len - 1];
        if !is_base62(last) {
            return Err(BmsTokenizeError::InvalidChannel {
                value: addr.to_owned(),
            });
        }
        if len >= 2 {
            let second_last = bytes[len - 2];
            if is_base62(second_last) {
                (&addr[len - 2..], &addr[..len - 2])
            } else {
                (&addr[len - 1..], &addr[..len - 1])
            }
        } else {
            (&addr[len - 1..], &addr[..len - 1])
        }
    };

    // Normalize to uppercase; channel IDs are case-insensitive and stored as
    // Base36 (uppercase alphanumerics).
    let channel_upper = channel_str.to_ascii_uppercase();
    let channel_idx: ChannelIndex =
        channel_upper
            .as_str()
            .try_into()
            .map_err(|_e| BmsTokenizeError::InvalidChannel {
                value: addr.to_owned(),
            })?;
    let channel = classify_channel(channel_idx);

    // Measure: extract all ASCII digits from prefix and build a u16.
    // Zero-based; an empty prefix means track = 0.
    let track: u16 = prefix
        .chars()
        .filter(char::is_ascii_digit)
        .fold(0u16, |acc, c| {
            acc.saturating_mul(10)
                .saturating_add(u16::from(c as u8 - b'0'))
        });

    Ok(Some(BmsMessage {
        addr: addr.to_owned(),
        body: body.to_owned(),
        track,
        channel,
    }))
}
