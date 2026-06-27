use bmsrs_chart::{Chart, ChartMetadata, Event, TimingTrack};

pub fn make_test_chart(events: Vec<Event<()>>) -> Chart {
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
