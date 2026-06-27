use bmsrs_chart::{
    Chart, ChartData, ChartInfo, Event, Lane, NoteKind, NoteSide, SongInfo, TimingTrack,
};

/// 测试用的默认参数事件类型。
type Evt = Event<()>;

pub fn make_chart(events: Vec<Evt>) -> Chart {
    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
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
        },
    }
}

pub const fn note(tick: u64, lane: Lane, kind: NoteKind) -> Evt {
    Event::Note {
        tick,
        side: NoteSide::P1,
        lane,
        kind,
        audio_index: None,
        ext: (),
    }
}
