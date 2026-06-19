#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;

use bmsrs_chart::mode::{Lane, PlayerSide};
use bmsrs_chart::note::{NoteData, NoteDataLike as _, NoteKind};

#[test]
fn default_note_data_side_lane_and_kind() {
    let data = NoteData {
        side: PlayerSide::Player1,
        lane: Lane::Key(NonZeroU8::new(5).unwrap()),
        kind: NoteKind::Long { duration: 480 },
    };
    assert_eq!(data.side(), PlayerSide::Player1);
    assert_eq!(data.lane(), Lane::Key(NonZeroU8::new(5).unwrap()));
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
