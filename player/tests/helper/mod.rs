use bmsrs_chart::{Bga, Chart, ChartMetadata, DefaultNoteData, Note, NoteKind, TimingTrack};

pub fn make_chart(notes: Vec<Note>) -> Chart {
    Chart {
        metadata: ChartMetadata::default(),
        resolution: 240,
        lane_count: 8,
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

pub fn note(tick: u64, lane: u16, kind: NoteKind) -> Note {
    Note {
        tick,
        audio: None,
        data: DefaultNoteData { lane, kind },
    }
}
