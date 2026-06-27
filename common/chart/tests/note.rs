#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;

use bmsrs_chart::{Damage, Event, Lane, NoteKind, NoteSide};

#[test]
fn event_note_fields() {
    let ev: Event<()> = Event::Note {
        tick: 240,
        side: NoteSide::P1,
        lane: Lane::Key(NonZeroU8::new(5).unwrap()),
        kind: NoteKind::Long { duration: 480 },
        audio_index: None,
        ext: (),
    };
    assert_eq!(ev.tick(), 240);
    match ev {
        Event::Note {
            side, lane, kind, ..
        } => {
            assert_eq!(side, NoteSide::P1);
            assert_eq!(lane, Lane::Key(NonZeroU8::new(5).unwrap()));
            assert_eq!(kind, NoteKind::Long { duration: 480 });
        }
        _ => panic!("expected Event::Note"),
    }
}

#[test]
fn note_kind_default_is_normal() {
    assert_eq!(NoteKind::default(), NoteKind::Normal);
}

#[test]
fn note_kind_clone_equal() {
    let kind = NoteKind::Mine {
        damage: Damage::new(12.5),
    };
    assert_eq!(kind.clone(), kind);
}
