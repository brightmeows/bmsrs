#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{AudioAsset, BgmEvent};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn audio_asset_bms_style() {
    let asset = AudioAsset {
        path: PathBuf::from("kick.wav"),
        start: Duration::ZERO,
        duration: None,
    };
    assert_eq!(asset.path, PathBuf::from("kick.wav"));
    assert_eq!(asset.start, Duration::ZERO);
    assert!(asset.duration.is_none());
}

#[test]
fn audio_asset_bmson_slice_style() {
    let asset = AudioAsset {
        path: PathBuf::from("vox.wav"),
        start: Duration::from_millis(250),
        duration: Some(Duration::from_millis(750)),
    };
    assert_eq!(asset.start, Duration::from_millis(250));
    assert_eq!(asset.duration, Some(Duration::from_millis(750)));
}

#[test]
fn bgm_event_fields() {
    let ev = BgmEvent {
        tick: 960,
        audio: 3,
    };
    assert_eq!(ev.tick, 960);
    assert_eq!(ev.audio, 3);
}
