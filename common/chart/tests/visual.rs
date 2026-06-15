#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BarLine, Bga, BgaResource, BgaTimelineEvent, ScrollChangeEvent};
use std::path::PathBuf;

#[test]
fn bar_line_tick() {
    let bl = BarLine { tick: 960 };
    assert_eq!(bl.tick, 960);
}

#[test]
fn scroll_change_fields() {
    let sc = ScrollChangeEvent {
        tick: 480,
        rate: 2.0,
    };
    assert_eq!(sc.tick, 480);
    assert!((sc.rate - 2.0).abs() < f64::EPSILON);
}

#[test]
fn bga_default_is_empty() {
    let bga = Bga::default();
    assert!(bga.resources.is_empty());
    assert!(bga.events.is_empty());
    assert!(bga.layer_events.is_empty());
    assert!(bga.poor_events.is_empty());
}

#[test]
fn bga_resource_fields() {
    let r = BgaResource {
        id: 5,
        path: PathBuf::from("bg.png"),
    };
    assert_eq!(r.id, 5);
}

#[test]
fn bga_timeline_event_fields() {
    let ev = BgaTimelineEvent {
        tick: 240,
        resource_id: 2,
    };
    assert_eq!(ev.tick, 240);
    assert_eq!(ev.resource_id, 2);
}
