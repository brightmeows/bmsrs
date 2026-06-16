//! BMS message (channel data) line parsing.
//!
//! # Format
//!
//! A channel data line has the form `#ADDR:body`, where:
//!
//! - `ADDR` is a string consisting of a **track** (numeric digits, 0-indexed)
//!   followed by a **channel** (the last 1–2 valid [`Base62`](crate::Base62)
//!   characters).  For example, in `#00111:...`, the address `00111` has
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

use crate::index::{Base62, BmsCharset, BmsIndex, ChannelTag};
use crate::{BmsStr, BmsToken, BmsTokenizeError, BmsTryFromError};

/// A channel data line in a BMS file (`#ADDR:body`).
///
/// See the module-level documentation for the format description.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsMessage<'a, C = &'a str> {
    /// Raw address string (before `:`).
    pub addr: C,
    /// Raw body string (after `:`).
    pub body: C,

    /// 0-indexed track number, extracted from the numeric digits in [`addr`](BmsMessage::addr)
    /// that precede the channel suffix.
    pub track: u16,
    /// Channel number — the last 1–2 valid [`Base62`](crate::Base62) characters
    /// from [`addr`](BmsMessage::addr).
    pub channel: BmsIndex<ChannelTag, Base62>,
    /// Phantom data to satisfy E0392 (unused lifetime parameter).
    pub _phantom: std::marker::PhantomData<&'a C>,
}

impl<'a, C: BmsStr<'a>> From<BmsMessage<'a, C>> for BmsToken<'a, C> {
    #[inline]
    fn from(msg: BmsMessage<'a, C>) -> Self {
        BmsToken::Message(msg)
    }
}

impl<'a, C: BmsStr<'a>> TryFrom<BmsToken<'a, C>> for BmsMessage<'a, C> {
    type Error = BmsTryFromError<'a>;

    #[inline]
    fn try_from(token: BmsToken<'a, C>) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Message(m) => Ok(m),
            _ => Err(BmsTryFromError::NotAMessage),
        }
    }
}

/// Attempts to parse a single line as a BMS channel message.
///
/// Returns `Ok(None)` if the line does not look like a channel message
/// (e.g., it is a header, a comment, or empty).
/// Returns `Err(...)` if the line looks like a channel message but has
/// no valid channel suffix.
pub(crate) fn parse_message_line<'a, C: Clone + AsRef<str> + fmt::Display + From<&'a str> + 'a>(
    line: &'a str,
) -> Result<Option<BmsMessage<'a, C>>, BmsTokenizeError<'a>> {
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
        if !Base62::is_valid(last) {
            return Err(BmsTokenizeError::InvalidChannel { value: addr });
        }
        if len >= 2 {
            let second_last = bytes[len - 2];
            if Base62::is_valid(second_last) {
                (&addr[len - 2..], &addr[..len - 2])
            } else {
                (&addr[len - 1..], &addr[..len - 1])
            }
        } else {
            (&addr[len - 1..], &addr[..len - 1])
        }
    };

    let channel: BmsIndex<ChannelTag, Base62> = channel_str
        .try_into()
        .map_err(|_| BmsTokenizeError::InvalidChannel { value: addr })?;

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
        _phantom: std::marker::PhantomData,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to avoid turbofish in test calls.
    fn parse_msg(s: &str) -> Result<Option<BmsMessage<'_, &str>>, BmsTokenizeError<'_>> {
        crate::message::parse_message_line(s)
    }

    fn ch(s: &str) -> BmsIndex<ChannelTag, Base62> {
        s.try_into().unwrap()
    }

    // Basic parsing

    #[test]
    fn parse_basic_message() {
        let result = parse_msg("#00111:11223344").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.addr, "00111");
        assert_eq!(msg.body, "11223344");
        assert_eq!(msg.track, 1);
        assert_eq!(msg.channel, ch("11"));
    }

    #[test]
    fn parse_high_track() {
        let result = parse_msg("#99908:FF").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.track, 999);
        assert_eq!(msg.channel, ch("08"));
        assert_eq!(msg.body, "FF");
    }

    #[test]
    fn parse_zero_track() {
        let result = parse_msg("#00051:A0B0").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.track, 0);
        assert_eq!(msg.channel, ch("51"));
    }

    #[test]
    fn parse_channel_0a() {
        let result = parse_msg("#0010A:01").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.track, 1);
        assert_eq!(msg.channel, ch("0A"));
        assert_eq!(msg.channel.as_u8_hex(), Some(10));
    }

    #[test]
    fn parse_channel_d1() {
        let result = parse_msg("#001D1:01").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.channel, ch("D1"));
        assert_eq!(msg.channel.as_u8_hex(), Some(209));
    }

    #[test]
    fn parse_channel_e9() {
        let result = parse_msg("#000E9:AA").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.channel, ch("E9"));
    }

    #[test]
    fn parse_channel_ff() {
        let result = parse_msg("#000FF:01").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.channel, ch("FF"));
        assert_eq!(msg.channel.as_u8_hex(), Some(255));
    }

    #[test]
    fn parse_extended_channel_sc() {
        let result = parse_msg("#000SC:1122").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.track, 0);
        assert_eq!(msg.channel, ch("SC"));
        // SC is not hex → as_u8_hex returns None
        assert_eq!(msg.channel.as_u8_hex(), None);
    }

    #[test]
    fn parse_extended_channel_sp() {
        let result = parse_msg("#001SP:AA").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.track, 1);
        assert_eq!(msg.channel, ch("SP"));
        assert_eq!(msg.channel.as_u8_hex(), None);
    }

    // Body & Track edge cases

    #[test]
    fn track_empty_prefix_is_zero() {
        let result = parse_msg("#01:1122").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.track, 0); // no prefix before channel "01"
        assert_eq!(msg.channel, ch("01"));
    }

    #[test]
    fn track_single_char_addr() {
        let result = parse_msg("#1:1122").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.track, 0); // prefix empty
        assert_eq!(msg.channel, ch("1"));
    }

    #[test]
    fn track_uses_only_digits_from_prefix() {
        let result = parse_msg("#0A01:1122").unwrap();
        let msg = result.expect("should parse");
        // addr="0A01", last 2="01"=channel, prefix="0A" → digits="0" → 0
        assert_eq!(msg.track, 0);
        assert_eq!(msg.channel, ch("01"));
    }

    // Return-Ok(None) cases

    #[test]
    fn empty_line_returns_none() {
        assert_eq!(parse_msg("").unwrap(), None);
    }

    #[test]
    fn no_hash_returns_none() {
        assert_eq!(parse_msg("hello world").unwrap(), None);
    }

    #[test]
    fn header_line_returns_none() {
        assert_eq!(parse_msg("#TITLE test").unwrap(), None);
    }

    #[test]
    fn comment_line_returns_none() {
        assert_eq!(parse_msg("// comment").unwrap(), None);
    }

    #[test]
    fn too_short_line_returns_none() {
        assert_eq!(parse_msg("#a").unwrap(), None);
        assert_eq!(parse_msg("#:").unwrap(), None);
    }

    #[test]
    fn addr_empty_returns_none() {
        assert_eq!(parse_msg("#:1122").unwrap(), None);
    }

    #[test]
    fn no_colon_returns_none() {
        assert_eq!(parse_msg("#00101").unwrap(), None);
    }

    // Error cases

    #[test]
    fn non_base62_last_char_returns_err() {
        // addr="001.." — last char '.' is not valid Base62
        let result = parse_msg("#001..:1122");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_channel_at_end_returns_err() {
        let result = parse_msg("#001!!:1122");
        assert!(result.is_err());
    }
}
