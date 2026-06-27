#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BgaLayer, BgaResource, Event};
use std::path::PathBuf;

#[test]
fn bar_event_tick() {
    let ev: Event<()> = Event::Bar { tick: 960 };
    assert_eq!(ev.tick(), 960);
}

#[test]
fn scroll_event_fields() {
    let ev: Event<()> = Event::Scroll {
        tick: 480,
        rate: 2.0,
    };
    assert_eq!(ev.tick(), 480);
    assert!(
        matches!(ev, Event::Scroll { rate, .. } if (rate - 2.0).abs() < f64::EPSILON),
        "expected Scroll with rate 2.0"
    );
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
fn bga_event_fields() {
    let ev: Event<()> = Event::Bga {
        tick: 240,
        layer: BgaLayer::Base,
        resource_id: 2,
    };
    assert_eq!(ev.tick(), 240);
    assert!(
        matches!(ev, Event::Bga { resource_id: 2, .. }),
        "expected Bga with resource_id 2"
    );
}
