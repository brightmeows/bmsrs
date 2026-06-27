//! Integration tests for [`BmsIndex`] and index newtypes.

use bms_tokenizer::*;

// BmsIndex tests

#[test]
fn bms_index_valid_two_char() {
    assert!(BmsIndex::try_from("01").is_ok());
    assert!(BmsIndex::try_from("ZZ").is_ok());
    assert!(BmsIndex::try_from("aa").is_ok());
    assert!(BmsIndex::try_from("zZ").is_ok());
    assert!(BmsIndex::try_from("FF").is_ok());
}

#[test]
fn bms_index_valid_one_char() {
    assert!(BmsIndex::try_from("A").is_ok());
    assert!(BmsIndex::try_from("9").is_ok());
    assert!(BmsIndex::try_from("0").is_ok());
}

#[test]
fn bms_index_empty_is_invalid() {
    assert!(BmsIndex::try_from("").is_err());
}

#[test]
fn bms_index_three_chars_is_invalid() {
    assert!(BmsIndex::try_from("AAA").is_err());
}

#[test]
fn bms_index_non_alphanumeric_is_invalid() {
    assert!(BmsIndex::try_from("*!").is_err());
    assert!(BmsIndex::try_from(" ").is_err());
    assert!(BmsIndex::try_from("ab:").is_err());
    assert!(BmsIndex::try_from("-1").is_err());
}

#[test]
fn as_str_returns_original() {
    let id = BmsIndex::try_from("2A").unwrap();
    assert_eq!(id.as_str(), "2A");
}

#[test]
fn as_str_one_char() {
    let id = BmsIndex::try_from("A").unwrap();
    assert_eq!(id.as_str(), "A");
}

#[test]
fn as_bytes_two_char() {
    let id = BmsIndex::try_from("2A").unwrap();
    assert_eq!(id.as_bytes(), b"2A");
}

#[test]
fn as_bytes_one_char() {
    let id = BmsIndex::try_from("A").unwrap();
    assert_eq!(id.as_bytes(), b"A");
}

#[test]
fn display_output() {
    let id = BmsIndex::try_from("FF").unwrap();
    assert_eq!(id.to_string(), "FF");
}

#[test]
fn to_index_base62() {
    assert_eq!(BmsIndex::try_from("00").unwrap().to_index(), Some(0));
    assert_eq!(BmsIndex::try_from("01").unwrap().to_index(), Some(1));
    assert_eq!(BmsIndex::try_from("ZZ").unwrap().to_index(), Some(1295));
    assert_eq!(BmsIndex::try_from("0A").unwrap().to_index(), Some(10));
}

#[test]
fn to_index_single_char() {
    assert_eq!(BmsIndex::try_from("A").unwrap().to_index(), Some(10));
    assert_eq!(BmsIndex::try_from("z").unwrap().to_index(), Some(35));
}

#[test]
fn as_u8_hex_two_digits() {
    assert_eq!(BmsIndex::try_from("0A").unwrap().as_u8_hex(), Some(10));
    assert_eq!(BmsIndex::try_from("FF").unwrap().as_u8_hex(), Some(255));
    assert_eq!(BmsIndex::try_from("D1").unwrap().as_u8_hex(), Some(209));
}

#[test]
fn as_u8_hex_one_digit() {
    assert_eq!(BmsIndex::try_from("A").unwrap().as_u8_hex(), Some(10));
    assert_eq!(BmsIndex::try_from("f").unwrap().as_u8_hex(), Some(15));
}

#[test]
fn size_of_bms_index() {
    assert_eq!(std::mem::size_of::<BmsIndex>(), 2);
}

// BmsBase tests

#[test]
fn is_valid_for_base62() {
    let id = BmsIndex::try_from("aZ").unwrap();
    assert!(id.is_valid_for(BmsBase::Base62));
    assert!(!id.is_valid_for(BmsBase::Base36));
    assert!(!id.is_valid_for(BmsBase::Base16));
}

#[test]
fn is_valid_for_base36() {
    let id = BmsIndex::try_from("AZ").unwrap();
    assert!(id.is_valid_for(BmsBase::Base62));
    assert!(id.is_valid_for(BmsBase::Base36));
    assert!(!id.is_valid_for(BmsBase::Base16));
}

#[test]
fn is_valid_for_base16() {
    let id = BmsIndex::try_from("AF").unwrap();
    assert!(id.is_valid_for(BmsBase::Base62));
    assert!(id.is_valid_for(BmsBase::Base36));
    assert!(id.is_valid_for(BmsBase::Base16));
}

// Newtype tests

#[test]
fn newtypes_are_distinct_types() {
    let wav: WavIndex = "01".parse().unwrap();
    let bmp: BmpIndex = "01".parse().unwrap();
    // Same string, same underlying value, different types.
    assert_eq!(wav.as_str(), bmp.as_str());
    // This line would not compile:
    // let _: WavIndex = bmp;
}

#[test]
fn wav_index_valid() {
    assert!(WavIndex::try_from("01").is_ok());
    assert!(WavIndex::try_from("aZ").is_ok());
}

#[test]
fn wav_index_invalid() {
    assert!(WavIndex::try_from("").is_err());
    assert!(WavIndex::try_from("AAA").is_err());
}

#[test]
fn channel_index_valid() {
    assert!(ChannelIndex::try_from("0A").is_ok());
    assert!(ChannelIndex::try_from("FF").is_ok());
    assert!(ChannelIndex::try_from("D1").is_ok());
}

#[test]
fn channel_index_rejects_lowercase() {
    assert!(ChannelIndex::try_from("aa").is_err());
    assert!(ChannelIndex::try_from("zZ").is_err());
    assert!(ChannelIndex::try_from("ff").is_err());
    assert!(ChannelIndex::try_from("gh").is_err());
}

#[test]
fn channel_index_as_u8_hex() {
    assert_eq!("0A".parse::<ChannelIndex>().unwrap().as_u8_hex(), Some(10));
    assert_eq!("FF".parse::<ChannelIndex>().unwrap().as_u8_hex(), Some(255));
}

#[test]
fn newtype_size_is_2_bytes() {
    assert_eq!(std::mem::size_of::<WavIndex>(), 2);
    assert_eq!(std::mem::size_of::<BmpIndex>(), 2);
    assert_eq!(std::mem::size_of::<ChannelIndex>(), 2);
}

#[test]
fn error_display() {
    let err = BmsIndexError {
        input: "!!!".to_owned(),
    };
    assert!(err.to_string().contains("!!!"));
}

#[test]
fn deref_provides_bms_index_methods() {
    let wav: WavIndex = "2A".parse().unwrap();
    // Via Deref<Target = BmsIndex>
    assert_eq!(wav.as_str(), "2A");
    assert_eq!(wav.as_bytes(), b"2A");
    assert_eq!(wav.to_string(), "2A");
    assert_eq!(wav.to_index(), Some(82));
}

#[test]
fn object_index_replaces_bms_object_id() {
    let obj: ObjectIndex = "ZZ".parse().unwrap();
    assert_eq!(obj.as_str(), "ZZ");
}
