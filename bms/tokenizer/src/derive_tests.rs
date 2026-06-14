//! Tests for `#[derive(BmsTokenAttr)]` — compiled as part of the library
//! so that `crate::` paths in generated code resolve correctly.
//! Covers all three modes: command, literal, dispatch.

use crate::{BmsIndex, BmsTokenAttr, BmsValue, WavTag};

#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
enum SimpleHeaders<'a> {
    #[bms_token("#TITLE {}")]
    Title(&'a str),
    #[bms_token("#BPM {}")]
    Bpm(f64),
    #[bms_token("#PLAYER {}")]
    Player(u64),
    #[bms_token("#GENRE")]
    Genre,
}

#[test]
fn command_unnamed_str() {
    let result = SimpleHeaders::try_match_header("TITLE", "My Song").unwrap();
    assert_eq!(result, Some(SimpleHeaders::Title("My Song")));
}

#[test]
fn command_unnamed_f64() {
    let result = SimpleHeaders::try_match_header("BPM", "180.0").unwrap();
    assert_eq!(result, Some(SimpleHeaders::Bpm(180.0)));
}

#[test]
fn command_unnamed_u64() {
    let result = SimpleHeaders::try_match_header("PLAYER", "1").unwrap();
    assert_eq!(result, Some(SimpleHeaders::Player(1)));
}

#[test]
fn command_unnamed_unit_variant() {
    let result = SimpleHeaders::try_match_header("GENRE", "").unwrap();
    assert_eq!(result, Some(SimpleHeaders::Genre));
}

#[test]
fn command_case_insensitive() {
    let result = SimpleHeaders::try_match_header("title", "lowercase").unwrap();
    assert_eq!(result, Some(SimpleHeaders::Title("lowercase")));
}

#[test]
fn command_format_unnamed() {
    let h = SimpleHeaders::Title("My Song");
    assert_eq!(
        h.format_header(),
        ("#TITLE".to_owned(), "My Song".to_owned())
    );
}

#[test]
fn command_format_unit_variant() {
    let h = SimpleHeaders::Genre;
    assert_eq!(h.format_header(), ("#GENRE".to_owned(), String::new()));
}

#[test]
fn command_no_match_returns_none() {
    let result = SimpleHeaders::try_match_header("UNKNOWN", "").unwrap();
    assert_eq!(result, None);
}

#[test]
fn command_invalid_int_returns_err() {
    let result = SimpleHeaders::try_match_header("PLAYER", "abc");
    assert!(result.is_err());
}

#[test]
fn command_invalid_float_returns_err() {
    let result = SimpleHeaders::try_match_header("BPM", "notanumber");
    assert!(result.is_err());
}

#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
enum NamedHeaders<'a> {
    #[bms_token("#WAV{id} {filename}")]
    Wav {
        id: BmsIndex<WavTag>,
        filename: &'a str,
    },
    #[bms_token("#BPM{id} {value}")]
    BpmDef { id: BmsIndex<WavTag>, value: f64 },
}

#[test]
fn command_named_indexed() {
    let result = NamedHeaders::try_match_header("WAV01", "kick.wav").unwrap();
    assert_eq!(
        result,
        Some(NamedHeaders::Wav {
            id: BmsIndex::<WavTag>::try_from("01").unwrap(),
            filename: "kick.wav",
        })
    );
}

#[test]
fn command_named_indexed_base36() {
    let result = NamedHeaders::try_match_header("WAV2A", "snare.wav").unwrap();
    assert_eq!(
        result,
        Some(NamedHeaders::Wav {
            id: BmsIndex::<WavTag>::try_from("2A").unwrap(),
            filename: "snare.wav",
        })
    );
}

#[test]
fn command_named_indexed_value() {
    let result = NamedHeaders::try_match_header("BPM01", "180.0").unwrap();
    assert_eq!(
        result,
        Some(NamedHeaders::BpmDef {
            id: BmsIndex::<WavTag>::try_from("01").unwrap(),
            value: 180.0,
        })
    );
}

#[test]
fn command_indexed_format() {
    let h = NamedHeaders::Wav {
        id: BmsIndex::<WavTag>::try_from("01").unwrap(),
        filename: "kick.wav",
    };
    assert_eq!(
        h.format_header(),
        ("#WAV01".to_owned(), "kick.wav".to_owned())
    );
}

#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
enum UnnamedIndexed {
    #[bms_token("#BPM{} {}")]
    BpmDef(BmsIndex<WavTag>, f64),
}

#[test]
fn command_unnamed_indexed_parse() {
    let result = UnnamedIndexed::try_match_header("BPM01", "180.0").unwrap();
    assert_eq!(
        result,
        Some(UnnamedIndexed::BpmDef(
            BmsIndex::<WavTag>::try_from("01").unwrap(),
            180.0,
        ))
    );
}

#[test]
fn command_unnamed_indexed_format() {
    let h = UnnamedIndexed::BpmDef(BmsIndex::<WavTag>::try_from("2A").unwrap(), 200.0);
    assert_eq!(h.format_header(), ("#BPM2A".to_owned(), "200".to_owned()));
}

#[derive(Debug, Clone, PartialEq)]
struct TestStp {
    measure: u16,
    duration_ms: f64,
}

impl std::fmt::Display for TestStp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:03} {}", self.measure, self.duration_ms)
    }
}

impl<'a> BmsValue<'a> for TestStp {
    fn parse(s: &'a str) -> Option<Self> {
        let (pos, dur) = s.split_once(' ')?;
        Some(Self {
            measure: pos.parse().ok()?,
            duration_ms: dur.parse().ok()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
enum FallbackHeaders<'a> {
    #[bms_token("#TITLE {}")]
    Title(&'a str),
    #[bms_token("#STP {params}")]
    #[bms_fallback]
    Stp { params: TestStp },
}

#[test]
fn command_fallback_valid_parses() {
    let result = FallbackHeaders::try_match_header("STP", "001 500").unwrap();
    assert_eq!(
        result,
        Some(FallbackHeaders::Stp {
            params: TestStp {
                measure: 1,
                duration_ms: 500.0,
            },
        })
    );
}

#[test]
fn command_fallback_invalid_returns_none() {
    let result = FallbackHeaders::try_match_header("STP", "invalid").unwrap();
    assert_eq!(result, None);
}

#[test]
fn command_non_fallback_valid() {
    let result = FallbackHeaders::try_match_header("TITLE", "hello").unwrap();
    assert_eq!(result, Some(FallbackHeaders::Title("hello")));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
enum TestMode {
    #[bms_token("1")]
    #[bms_token("01")]
    One,
    #[bms_token("2")]
    Two,
    #[bms_token("SP")]
    #[bms_token("sp")]
    SinglePlay,
}

#[test]
fn literal_parse_exact() {
    assert_eq!("1".parse::<TestMode>().unwrap(), TestMode::One);
    assert_eq!("01".parse::<TestMode>().unwrap(), TestMode::One);
    assert_eq!("2".parse::<TestMode>().unwrap(), TestMode::Two);
}

#[test]
fn literal_parse_case_sensitive() {
    assert_eq!("SP".parse::<TestMode>().unwrap(), TestMode::SinglePlay);
    assert_eq!("sp".parse::<TestMode>().unwrap(), TestMode::SinglePlay);
    assert!("Sp".parse::<TestMode>().is_err());
}

#[test]
fn literal_invalid_returns_err() {
    assert!("3".parse::<TestMode>().is_err());
    assert!("".parse::<TestMode>().is_err());
}

#[test]
fn literal_display_first_token() {
    assert_eq!(TestMode::One.to_string(), "1");
    assert_eq!(TestMode::Two.to_string(), "2");
    assert_eq!(TestMode::SinglePlay.to_string(), "SP");
}
