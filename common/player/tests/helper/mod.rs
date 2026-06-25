use bmsrs_chart::{
    Bga, Chart, ChartMetadata, Lane, Note, NoteData, NoteKind, NoteSide, TimingTrack,
};

pub fn make_chart(notes: Vec<Note>) -> Chart {
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
        notes,
        bgm: vec![],
        audio_assets: vec![],
        bar_lines: vec![],
        scroll_events: vec![],
        bga: Bga::default(),
    }
}

pub const fn note(tick: u64, lane: Lane, kind: NoteKind) -> Note {
    Note {
        tick,
        audio: None,
        data: NoteData {
            side: NoteSide::P1,
            lane,
            kind,
        },
    }
}
