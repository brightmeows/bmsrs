use bmsrs_chart::{
    Chart, ChartData, ChartInfo, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint,
    NoteKind, NoteSide, SongInfo, TimingTrack,
};

/// 测试用的默认参数事件类型。
type Evt = Event<()>;

pub fn make_chart(events: Vec<Evt>) -> Chart {
    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::simple(120.0),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events,
            judge_deltas: None,
            life_deltas: None,
            audio_assets: vec![],
        },
    }
}

pub const fn note(tick: u64, lane: Lane, kind: NoteKind) -> Evt {
    Event::new(
        tick,
        EventKind::Note {
            side: NoteSide::P1,
            lane,
            kind,
            audio_index: None,
            ext: (),
        },
    )
}
