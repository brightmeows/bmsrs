#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BgaLayer, BgaResource, Event, EventKind};
use std::path::PathBuf;

#[test]
fn bar_event_tick() {
    let ev: Event<()> = Event::bar(960);
    assert_eq!(ev.tick(), 960);
}

#[test]
fn scroll_event_fields() {
    let ev: Event<()> = Event::scroll(480, 2.0);
    assert_eq!(ev.tick(), 480);
    assert!(
        matches!(ev.kind, EventKind::Scroll { rate, .. } if (rate - 2.0).abs() < f64::EPSILON),
        "expected Scroll with rate 2.0"
    );
}

#[test]
fn bga_resource_fields() {
    let r = BgaResource {
        id: 5,
        path: PathBuf::from("bg.png"),
        crop: None,
    };
    assert_eq!(r.id, 5);
    assert!(r.crop.is_none());
}

#[test]
fn bga_event_fields() {
    let ev: Event<()> = Event::new(
        240,
        EventKind::Bga {
            layer: BgaLayer::Base,
            resource_id: 2,
        },
    );
    assert_eq!(ev.tick(), 240);
    assert!(
        matches!(ev.kind, EventKind::Bga { resource_id: 2, .. }),
        "expected Bga with resource_id 2"
    );
}
