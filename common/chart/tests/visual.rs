#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BgaLayer, BgaResourceContent, BgaResourceId, Event, EventKind};
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
    let r = BgaResourceContent::File {
        path: PathBuf::from("bg.png"),
        crop: None,
    };
    assert!(matches!(r, BgaResourceContent::File { crop: None, .. }));
    assert_eq!(r.file_kind(), Some(bmsrs_chart::BgaResourceKind::Static));
}

#[test]
fn bga_event_fields() {
    let ev: Event<()> = Event::new(
        240,
        EventKind::Bga {
            layer: BgaLayer::Base,
            resource_id: Some(BgaResourceId::new(2)),
        },
    );
    assert_eq!(ev.tick(), 240);
    assert!(
        matches!(
            ev.kind,
            EventKind::Bga {
                resource_id: Some(resource_id),
                ..
            } if resource_id.get() == 2
        ),
        "expected Bga with resource_id 2"
    );
}
