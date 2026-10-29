use bmsrs_chart::{
    Chart, ChartData, ChartInfo, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint,
    NoteAudio, NoteKind, NoteRole, NoteSide, SongInfo, TimingTrack,
};

/// Default-parameter event type for tests.
type Evt = Event<()>;

pub fn make_chart(events: Vec<Evt>) -> Chart {
    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            timing: TimingTrack::simple(120.0, 240).unwrap(),
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
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
            role: NoteRole::Normal, audio: NoteAudio::none(),
            ext: (),
        },
    )
}
