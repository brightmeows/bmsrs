//! BMS message (channel data) line parsing.
//!
//! # Format
//!
//! A channel data line has the form `#ADDR:body`, where:
//!
//! - `ADDR` is a string consisting of a **track** (numeric digits, 0-indexed)
//!   followed by a **channel** (the last 1–2 valid Base62 characters
//!   (`0-9A-Za-z`)).  For example, in `#00111:...`, the address `00111` has
//!   track `001` and channel `11`.
//! - `body` is the raw value string — 2-character object indices are parsed
//!   by downstream (parser) from concatenated raw storage.
//!   Unknown or extended channels (e.g., `SC`, `SP`, `1G`) are preserved in
//!   the [`channel`](BmsMessage::channel) field and in raw storage.
//!
//! # Channel semantics (selected)
//!
//! Full channel mapping is the parser's responsibility — this table
//! only lists the most common channels for reference:
//!
//! | Channel | Purpose |
//! |---------|---------|
//! | `01` | BGM (can span multiple lines) |
//! | `02` | Measure length change |
//! | `03` | BPM change (hex integer, `[01-FF]`) |
//! | `04` | BGA BASE layer |
//! | `06` | BGA POOR (miss) layer |
//! | `07` | BGA LAYER (black = transparent) |
//! | `08` | Extended BPM change |
//! | `09` | STOP sequence |
//! | `0A` | BGA LAYER2 (SCROLL) |
//! | `11-19` | 1P visible notes |
//! | `21-29` | 2P visible notes |
//! | `31-39` | 1P invisible notes |
//! | `41-49` | 2P invisible notes |
//! | `51-69` | Long note channels |
//! | `D1-D9` | 1P landmines |
//! | `E1-E9` | 2P landmines |
//! | `SC` | SCROLL (extended) |
//! | `SP` | SPEED (extended) |
//!
//! # Examples
//!
//! ```text
//! #00111:11223344 → addr="00111", body="11223344"
//!                    track=1, channel="11"
//! #0010A:01       → addr="0010A", body="01"
//!                    track=1, channel="0A"
//! #000SC:         → addr="000SC", body=""
//!                    track=0, channel="SC"
//! ```

use std::fmt;

use crate::channel::{BmsChannel, classify_channel};
use crate::index::{ChannelIndex, is_base62};
use crate::{BmsToken, BmsTokenizeError, BmsTryFromError};

/// A channel data line in a BMS file (`#ADDR:body`).
///
/// See the module-level documentation for the format description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsMessage<C> {
    /// Raw address string (before `:`).
    pub addr: C,
    /// Raw body string (after `:`).
    pub body: C,

    /// 0-indexed track number, extracted from the numeric digits in [`addr`](BmsMessage::addr)
    /// that precede the channel suffix.
    pub track: u16,
    /// Channel number — the last 1–2 valid Base62 characters (`0-9A-Za-z`)
    /// from [`addr`](BmsMessage::addr), categorised into a [`BmsChannel`] enum.
    pub channel: BmsChannel,
}

impl<C> TryFrom<BmsToken<C>> for BmsMessage<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(token: BmsToken<C>) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Message(m) => Ok(m),
            BmsToken::Header(_) => Err(BmsTryFromError::NotAMessage),
        }
    }
}

/// Attempts to parse a single line as a BMS channel message.
///
/// Returns `Ok(None)` if the line does not look like a channel message
/// (e.g., it is a header, a comment, or empty).
/// Returns `Err(...)` if the line looks like a channel message but has
/// no valid channel suffix.
///
/// # Errors
///
/// Returns [`BmsTokenizeError::InvalidChannel`] when the address has a
/// non-Base62 character at the channel position or an unrecognisable
/// channel suffix.
#[expect(
    clippy::string_slice,
    reason = "BMS message lines are ASCII-only (hex digits, Base62 chars, colons); byte indexing is safe"
)]
pub fn parse_message_line<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a>(
    line: &'a str,
) -> Result<Option<BmsMessage<C>>, BmsTokenizeError<C>> {
    if line.is_empty() || !line.starts_with('#') {
        return Ok(None);
    }

    // Need at least: # + 1 char + :  (e.g., "#1:")
    if line.len() < 3 {
        return Ok(None);
    }

    let rest = &line[1..]; // strip '#'

    // Split at ':' to get addr and body
    let Some((addr, body)) = rest.split_once(':') else {
        return Ok(None); // No colon — not a message line
    };

    if addr.is_empty() || !addr.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        // addr must start with a digit (track number prefix) to be a message
        // line; otherwise it's a header like #SWBGA01 30:... that happens to
        // contain a colon.
        return Ok(None);
    }

    // Parse channel from the end of addr:
    // take the last 1–2 consecutive valid Base62 characters.
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
                value: C::from(addr),
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

    // Normalise to uppercase; channel IDs are case-insensitive and stored
    // as Base36 (uppercase alphanumeric).
    let channel_upper = channel_str.to_ascii_uppercase();
    let channel_idx: ChannelIndex =
        channel_upper
            .as_str()
            .try_into()
            .map_err(|_e| BmsTokenizeError::InvalidChannel {
                value: C::from(addr),
            })?;
    let channel = classify_channel(channel_idx);

    // Track: extract all ASCII digit characters from prefix, build u16.
    // 0-indexed; empty prefix → track = 0.
    let track: u16 = prefix
        .chars()
        .filter(char::is_ascii_digit)
        .fold(0u16, |acc, c| {
            acc.saturating_mul(10)
                .saturating_add(u16::from(c as u8 - b'0'))
        });

    Ok(Some(BmsMessage {
        addr: C::from(addr),
        body: C::from(body),
        track,
        channel,
    }))
}
