use bmsrs_chart::{Chart, ChartData, ChartInfo, Event, SongInfo, TimingTrack};

pub fn make_test_chart(events: Vec<Event<()>>) -> Chart {
    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(120.0, vec![], vec![]),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            events,
            audio_assets: vec![],
        },
    }
}
