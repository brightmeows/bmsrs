#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;

use bmsrs_chart::{
    AudioIndex, Damage, Event, EventKind, Lane, NoteAudio, NoteKind, NoteRole, NoteSide,
};

#[test]
fn event_note_fields() {
    let ev: Event<()> = Event::new(
        240,
        EventKind::Note {
            side: NoteSide::P1,
            lane: Lane::Key(NonZeroU8::new(5).unwrap()),
            kind: NoteKind::Long { duration: 480 },
            role: NoteRole::Normal,
            audio: NoteAudio::none(),
            ext: (),
        },
    );
    assert_eq!(ev.tick(), 240);
    match ev.kind {
        EventKind::Note {
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
fn note_audio_from_indices_collapses_single() {
    let audio = NoteAudio::from_indices([AudioIndex::new(7)]);

    assert_eq!(audio.primary(), Some(AudioIndex::new(7)));
}

#[test]
fn note_audio_from_indices_preserves_layers() {
    let audio = NoteAudio::from_indices([AudioIndex::new(7), AudioIndex::new(9)]);

    assert_eq!(audio.len(), 2);
}

#[test]
fn note_role_default_is_normal() {
    assert_eq!(NoteRole::default(), NoteRole::Normal);
}

#[test]
fn note_kind_clone_equal() {
    let kind = NoteKind::Mine {
        damage: Damage::new(12.5),
    };
    assert_eq!(kind.clone(), kind);
}
