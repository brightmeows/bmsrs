//! BMS message (channel data) line parsing.

use crate::BmsTokenizeError;
use crate::id::{BmsChannelId, ChannelTag, Hex};

/// A channel data line in a BMS file (`#xxxYY:values`).
///
/// # Format
///
/// `#` + 3-digit measure + 2-character hex channel + `:` + value string
///
/// # Examples
///
/// `#00111:11223344` → measure=1, channel="11", values="11223344"
/// `#0010A:01`       → measure=1, channel="0A" (EXRANK), values="01"
#[derive(Debug, Clone, PartialEq)]
pub struct BmsMessage<'a> {
    /// Measure number (0–999).
    pub measure: u16,
    /// Channel number as a validated hex ID.
    pub channel: BmsChannelId<ChannelTag, Hex>,
    /// Raw value string (sequence of 2-character object indices).
    pub values: &'a str,
}

/// Attempts to parse a single line as a BMS channel message.
///
/// Returns `Ok(None)` if the line does not look like a channel message
/// (e.g., it is a header, a comment, or empty).
/// Returns `Err(...)` if the line looks like a channel message but has
/// an invalid measure or channel number.
pub(crate) fn parse_message_line(
    line: &str,
) -> Result<Option<BmsMessage<'_>>, BmsTokenizeError<'_>> {
    if line.is_empty() || !line.starts_with('#') {
        return Ok(None);
    }

    // Must have at least: # + 3 digits + 2 chars + : = 7 chars before value
    if line.len() < 7 {
        return Ok(None);
    }

    let rest = &line[1..]; // strip '#'

    // Rest must be: xxxYY:values
    let measure_str = &rest[..3];
    if !measure_str.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(None);
    }

    let (channel_str, values_str) =
        rest[3..]
            .split_once(':')
            .ok_or(BmsTokenizeError::InvalidChannel {
                value: "missing colon",
            })?;

    // 3 decimal digits (000–999) always fit in u16.
    // The `?` operator previously used From<ParseIntError> which is now removed;
    // we map the error to InvalidMeasure for consistency.
    let measure: u16 = measure_str
        .parse()
        .map_err(|_| BmsTokenizeError::InvalidMeasure { value: measure_str })?;

    // Channel is 2 hex characters (e.g., "0A", "D1", "11").
    let channel: BmsChannelId<ChannelTag, Hex> = channel_str
        .try_into()
        .map_err(|_| BmsTokenizeError::InvalidChannel { value: channel_str })?;

    Ok(Some(BmsMessage {
        measure,
        channel,
        values: values_str,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(s: &str) -> BmsChannelId<ChannelTag, Hex> {
        s.try_into().unwrap()
    }

    #[test]
    fn parse_basic_message() {
        let result = parse_message_line("#00111:11223344").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 1);
        assert_eq!(msg.channel, ch("11"));
        assert_eq!(msg.values, "11223344");
    }

    #[test]
    fn parse_high_measure() {
        let result = parse_message_line("#99908:FF").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 999);
        assert_eq!(msg.channel, ch("08"));
        assert_eq!(msg.values, "FF");
    }

    #[test]
    fn parse_zero_measure() {
        let result = parse_message_line("#00051:A0B0").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 0);
        assert_eq!(msg.channel, ch("51"));
        assert_eq!(msg.values, "A0B0");
    }

    #[test]
    fn parse_hex_channel_0a() {
        let result = parse_message_line("#0010A:01").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 1);
        assert_eq!(msg.channel, ch("0A"));
        assert_eq!(msg.channel.as_u8_hex(), Some(10));
        assert_eq!(msg.values, "01");
    }

    #[test]
    fn parse_hex_channel_d1() {
        let result = parse_message_line("#001D1:01").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.channel, ch("D1"));
        assert_eq!(msg.channel.as_u8_hex(), Some(209));
    }

    #[test]
    fn parse_hex_channel_e9() {
        let result = parse_message_line("#000E9:AA").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.channel, ch("E9"));
    }

    #[test]
    fn parse_hex_channel_ff() {
        let result = parse_message_line("#000FF:01").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.channel, ch("FF"));
        assert_eq!(msg.channel.as_u8_hex(), Some(255));
    }

    #[test]
    fn parse_with_varied_values() {
        let result = parse_message_line("#00101:00112233445566778899AABBCCDDEEFFZZ").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 1);
        assert_eq!(msg.channel, ch("01"));
        assert_eq!(msg.values, "00112233445566778899AABBCCDDEEFFZZ");
    }

    #[test]
    fn empty_line_returns_none() {
        assert_eq!(parse_message_line("").unwrap(), None);
    }

    #[test]
    fn whitespace_only_line_returns_none() {
        assert_eq!(parse_message_line("   ").unwrap(), None);
    }

    #[test]
    fn header_line_returns_none() {
        assert_eq!(parse_message_line("#TITLE test").unwrap(), None);
    }

    #[test]
    fn comment_line_returns_none() {
        assert_eq!(parse_message_line("// comment").unwrap(), None);
    }

    #[test]
    fn too_short_line_returns_none() {
        assert_eq!(parse_message_line("#abc").unwrap(), None);
    }

    #[test]
    fn missing_colon_after_channel() {
        let result = parse_message_line("#00111001122");
        assert!(result.is_err());
    }

    #[test]
    fn non_digit_measure_returns_none() {
        assert_eq!(parse_message_line("#abc11:1122").unwrap(), None);
    }

    #[test]
    fn non_hex_channel_returns_err() {
        let result = parse_message_line("#001GZ:1122");
        assert!(result.is_err());
    }

    #[test]
    fn measure_accepts_any_three_digits() {
        let result = parse_message_line("#10001:1122").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 100);
        assert_eq!(msg.channel, ch("01"));
        assert_eq!(msg.values, "1122");
    }

    #[test]
    fn values_can_be_empty_after_colon() {
        let result = parse_message_line("#00111:").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 1);
        assert_eq!(msg.channel, ch("11"));
        assert_eq!(msg.values, "");
    }

    #[test]
    fn debug_format() {
        let msg = BmsMessage {
            measure: 1,
            channel: ch("11"),
            values: "1122",
        };
        let debug = format!("{msg:?}");
        assert!(debug.contains("measure: 1"));
        assert!(debug.contains("values: \"1122\""));
    }
}
