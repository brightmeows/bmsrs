use bmsrs_chart::{Chart, ChartData, ChartInfo, Event, SongInfo};

pub fn make_test_chart(events: Vec<Event<()>>) -> Chart {
    let data = ChartData::new(120.0, 240, events, vec![])
        .unwrap()
        .with_judge_multiplier(Some(1.0))
        .with_life_multiplier(Some(1.0));
    Chart::new(SongInfo::default(), ChartInfo::default(), data).unwrap()
}
