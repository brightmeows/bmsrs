use bmsrs_chart::{
    Chart, ChartData, ChartInfo, Event, EventKind, Lane, NoteAudio, NoteKind, NoteRole, NoteSide,
    SongInfo,
};

/// Default-parameter event type for tests.
type Evt = Event<()>;

pub fn make_chart(events: Vec<Evt>) -> Chart {
    let data = ChartData::new(120.0, 240, events, vec![])
        .unwrap()
        .with_judge_multiplier(Some(1.0))
        .with_life_multiplier(Some(1.0));
    Chart::new(SongInfo::default(), ChartInfo::default(), data).unwrap()
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
