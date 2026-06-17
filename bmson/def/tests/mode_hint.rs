#![expect(missing_docs, reason = "integration tests")]

use bmson_def::ModeHint;

#[test]
fn round_trip_standard() {
    for hint in [
        ModeHint::Beat5k,
        ModeHint::Beat7k,
        ModeHint::Beat10k,
        ModeHint::Beat14k,
        ModeHint::Popn5k,
        ModeHint::Popn9k,
        ModeHint::Dj5kOnly,
        ModeHint::DjRuby,
        ModeHint::Dj5k,
        ModeHint::Dj7k,
        ModeHint::Dj10k,
        ModeHint::Dj14k,
        ModeHint::DjAndromeda,
    ] {
        let ser = hint.to_string();
        let de: ModeHint = ser.parse().unwrap();
        assert_eq!(hint, de, "failed round-trip for {hint}");
    }
}

#[test]
fn generic_spec_format() {
    let de: ModeHint = "generic-5keys".parse().unwrap();
    assert_eq!(de, ModeHint::Generic(5));
    assert_eq!(de.to_string(), "generic-5keys");
}

#[test]
fn generic_legacy_compat() {
    let de: ModeHint = "generic-7k".parse().unwrap();
    assert_eq!(de, ModeHint::Generic(7));
    assert_eq!(de.to_string(), "generic-7keys");
}

#[test]
fn other_passthrough() {
    let de: ModeHint = "custom-mode-42".parse().unwrap();
    assert_eq!(de, ModeHint::Other("custom-mode-42".to_owned()));
}

#[test]
fn generic_parse_fallback() {
    let de: ModeHint = "generic-abc".parse().unwrap();
    assert_eq!(de, ModeHint::Other("generic-abc".to_owned()));
}

#[test]
fn serde_json_round_trip() {
    let original = ModeHint::Dj14k;
    let json = serde_json::to_string(&original).unwrap();
    let de: ModeHint = serde_json::from_str(&json).unwrap();
    assert_eq!(original, de);

    let generic = ModeHint::Generic(12);
    let generic_json = serde_json::to_string(&generic).unwrap();
    assert_eq!(generic_json, "\"generic-12keys\"");
    let generic_de: ModeHint = serde_json::from_str(&generic_json).unwrap();
    assert_eq!(generic, generic_de);
}
