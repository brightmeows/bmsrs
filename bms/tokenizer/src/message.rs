//! BMS message (channel data) line parsing.

use crate::error::BmsTokenizeError;

/// A channel data line in a BMS file (`#xxxYY:values`).
///
/// # Format
///
/// `#` + 3-digit measure + 2-digit channel + `:` + value string
///
/// # Examples
///
/// `#00111:11223344` → measure=1, channel=11, values="11223344"
#[derive(Debug, Clone, PartialEq)]
pub struct BmsMessage<'a> {
    /// Measure number (0–999).
    pub measure: u16,
    /// Channel number (01–99, but typically 01–E9 in hex notation).
    pub channel: u8,
    /// Raw value string (sequence of 2-character object indices).
    pub values: &'a str,
}

/// Attempts to parse a single line as a BMS channel message.
///
/// Returns `Ok(None)` if the line does not look like a channel message
/// (e.g., it is a header, a comment, or empty).
/// Returns `Err(...)` if the line looks like a channel message but has
/// an invalid measure or channel number.
pub(crate) fn parse_message_line(line: &str) -> Result<Option<BmsMessage<'_>>, BmsTokenizeError> {
    let trimmed = line.trim();

    if trimmed.is_empty() || !trimmed.starts_with('#') {
        return Ok(None);
    }

    // Must have at least: # + 3 digits + 2 digits + : = 7 chars before value
    if trimmed.len() < 7 {
        return Ok(None);
    }

    let rest = &trimmed[1..]; // strip '#'

    // Rest must be: xxxYY:values
    let measure_str = &rest[..3];
    if !measure_str.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(None);
    }

    let (channel_str, values_str) =
        rest[3..]
            .split_once(':')
            .ok_or(BmsTokenizeError::InvalidChannel(
                "missing colon".to_string(),
            ))?;

    if !channel_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(BmsTokenizeError::InvalidChannel(channel_str.to_string()));
    }

    // Safe because we validated digits above — 3 decimal digits always fit in u16.
    let measure: u16 = measure_str.parse().expect("measure digits validated above");

    // Safe because we validated digits above — 2 decimal digits always fit in u8.
    let channel: u8 = channel_str.parse().expect("channel digits validated above");

    Ok(Some(BmsMessage {
        measure,
        channel,
        values: values_str,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_message() {
        let result = parse_message_line("#00111:11223344").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 1);
        assert_eq!(msg.channel, 11);
        assert_eq!(msg.values, "11223344");
    }

    #[test]
    fn parse_high_measure() {
        let result = parse_message_line("#99908:FF").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 999);
        assert_eq!(msg.channel, 08);
        assert_eq!(msg.values, "FF");
    }

    #[test]
    fn parse_zero_measure() {
        let result = parse_message_line("#00051:A0B0").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 0);
        assert_eq!(msg.channel, 51);
        assert_eq!(msg.values, "A0B0");
    }

    #[test]
    fn parse_with_varied_values() {
        let result = parse_message_line("#00101:00112233445566778899AABBCCDDEEFFZZ").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 1);
        assert_eq!(msg.channel, 01);
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
    fn non_digit_channel_returns_err() {
        let result = parse_message_line("#001ab:1122");
        assert!(result.is_err());
    }

    #[test]
    fn measure_accepts_any_three_digits() {
        let result = parse_message_line("#100011:1122").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 100);
        assert_eq!(msg.channel, 11);
        assert_eq!(msg.values, "1122");
    }

    #[test]
    fn values_can_be_empty_after_colon() {
        let result = parse_message_line("#00111:").unwrap();
        let msg = result.expect("should parse");
        assert_eq!(msg.measure, 1);
        assert_eq!(msg.channel, 11);
        assert_eq!(msg.values, "");
    }

    #[test]
    fn debug_format() {
        let msg = BmsMessage {
            measure: 1,
            channel: 11,
            values: "1122",
        };
        let debug = format!("{msg:?}");
        assert!(debug.contains("measure: 1"));
        assert!(debug.contains("channel: 11"));
        assert!(debug.contains("values: \"1122\""));
    }
}
