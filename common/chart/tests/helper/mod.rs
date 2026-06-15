use bmsrs_chart::{Bga, BgmEvent, Chart, ChartMetadata, Note, TimingTrack};

pub fn make_test_chart(notes: Vec<Note>, bgm: Vec<BgmEvent>) -> Chart {
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
        bgm,
        audio_assets: vec![],
        bar_lines: vec![],
        scroll_events: vec![],
        bga: Bga::default(),
    }
}
