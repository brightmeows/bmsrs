#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::note::{DefaultNoteData, NoteData, NoteKind};

#[test]
fn default_note_data_lane_and_kind() {
    let data = DefaultNoteData {
        lane: 5,
        kind: NoteKind::Long { duration: 480 },
    };
    assert_eq!(data.lane(), 5);
    assert_eq!(data.kind(), NoteKind::Long { duration: 480 });
}

#[test]
fn note_kind_default_is_normal() {
    assert_eq!(NoteKind::default(), NoteKind::Normal);
}

#[test]
fn note_kind_clone_equal() {
    let kind = NoteKind::Mine { damage: 12.5 };
    assert_eq!(kind.clone(), kind);
}
