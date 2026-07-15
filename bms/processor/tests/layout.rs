#![expect(missing_docs, reason = "integration test")]

use bms_parser::{Bms, KeyType, LongNoteEvent, MineEvent, NoteEvent, Position};
use bms_processor::layout::{self, PmsLayout};

#[test]
fn detect_standard_pms_from_player2_channels() {
    let mut bms = Bms::default();
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 1),
        player: 2,
        lane: 3,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    assert_eq!(layout::detect_pms_variant(&bms), PmsLayout::Standard);
}

#[test]
fn detect_bme_type_pms_from_player1_channels() {
    let mut bms = Bms::default();
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 1),
        player: 1,
        lane: 7,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    assert_eq!(layout::detect_pms_variant(&bms), PmsLayout::BmeType);
}

#[test]
fn standard_defaults_when_no_pms_channels() {
    let bms = Bms::default();
    assert_eq!(layout::detect_pms_variant(&bms), PmsLayout::Standard);
}

#[test]
fn standard_from_long_note_player2_channels() {
    let mut bms = Bms::default();
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 1),
        player: 2,
        lane: 4,
        wav_id: "01".parse().unwrap(),
    });
    assert_eq!(layout::detect_pms_variant(&bms), PmsLayout::Standard);
}

#[test]
fn bme_type_from_mine_player1_channels() {
    let mut bms = Bms::default();
    bms.messages.mine_events.push(MineEvent {
        position: Position::new(0, 0, 1),
        player: 1,
        lane: 8,
        damage: 1.0,
    });
    assert_eq!(layout::detect_pms_variant(&bms), PmsLayout::BmeType);
}

#[test]
fn both_standard_and_bme_prefers_standard() {
    let mut bms = Bms::default();
    // Standard-indicating: player 2, lane 3
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 1),
        player: 2,
        lane: 3,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    // BME-type-indicating: player 1, lane 7
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 1),
        player: 1,
        lane: 7,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    assert_eq!(layout::detect_pms_variant(&bms), PmsLayout::Standard);
}
