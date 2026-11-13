//! bmson v0.2 legacy `bga_sequence` tests: deserialization and version carry.

use bmsrs_bmson_model::*;

#[test]
fn bga_sequence_deserializes_and_defaults_to_none() {
    let with_clips: BGA = serde_json::from_str(
        r#"{
            "bga_header": [{"id": 1, "name": "a.png"}],
            "bga_events": [],
            "layer_events": [],
            "poor_events": [],
            "bga_sequence": [
                {"id": 7, "sequence": [
                    {"time": 250, "id": 1},
                    {"time": 250, "id": null}
                ]}
            ]
        }"#,
    )
    .unwrap();
    let clips = with_clips.bga_sequence.expect("clips present");
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, 7);
    assert_eq!(clips[0].sequence.len(), 2);
    assert_eq!(clips[0].sequence[0].id, Some(1));
    assert_eq!(clips[0].sequence[1].id, None, "null id is a blank frame");
    assert_eq!(clips[0].sequence[1].time, 250);

    let without: BGA = serde_json::from_str(
        r#"{"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}"#,
    )
    .unwrap();
    assert_eq!(without.bga_sequence, None);
}

#[test]
fn v0_bga_sequence_carries_through_v2_upgrade() {
    let v0_json = r#"{
        "version": "0.2.1",
        "info": {"title": "T", "artist": "A", "genre": "G", "level": 8, "initBPM": 120.0},
        "bga": {
            "bga_header": [{"id": 1, "name": "a.png"}],
            "bga_sequence": [
                {"id": 7, "sequence": [{"time": 250, "id": 1}]}
            ],
            "bga_events": [],
            "layer_events": [{"y": 240, "id": 1}],
            "poor_events": [{"y": 0, "id": 7}]
        }
    }"#;
    let v0: v0::Bmson = serde_json::from_str(v0_json).unwrap();
    let v2 = Bmson::try_from(v0).unwrap();
    let clips = v2
        .chart_info
        .bga
        .bga_sequence
        .expect("clips carried through the upgrade");
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, 7);
    assert_eq!(clips[0].sequence[0].id, Some(1));
}
