use bmsrs_chart::{
    Chart, ChartData, ChartInfo, Event, LnJudgeHint, LnLifeHint, LnTypeHint, SongInfo,
};

pub fn make_test_chart(events: Vec<Event<()>>) -> Chart {
    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            init_bpm: 120.0,
            resolution: 240,
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events,
            audio_assets: vec![],
        },
    }
}
