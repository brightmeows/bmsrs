use bmsrs_chart::{Chart, ChartMetadata, Event, Lane, NoteKind, NoteSide, TimingTrack};

/// Event type with default params for tests.
type Evt = Event<()>;

pub fn make_chart(events: Vec<Evt>) -> Chart {
    Chart {
        metadata: ChartMetadata::default(),
        resolution: 240,
        timing: TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        },
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        events,
        audio_assets: vec![],
        bga_resources: vec![],
    }
}

pub const fn note(tick: u64, lane: Lane, kind: NoteKind) -> Evt {
    Event::Note {
        tick,
        side: NoteSide::P1,
        lane,
        kind,
        audio_index: None,
        ext: (),
    }
}
