//! Visual elements: bar lines, scroll-speed changes, and BGA.

use std::path::PathBuf;

/// A bar line position for visual display.
///
/// Bar lines are display hints — they do not affect gameplay timing.
/// The processor generates them from the source format's measure structure
/// (BMS) or explicit `lines` array (BMSON).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarLine {
    /// Tick position.
    pub tick: u64,
}

/// A scroll-speed multiplier change event.
///
/// The scroll rate affects how fast notes approach the judgement line
/// on screen. A rate of `1.0` is the default speed; negative values
/// cause reverse scroll.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollChangeEvent {
    /// Tick position.
    pub tick: u64,
    /// Speed multiplier (`1.0` = normal, negative = reverse).
    pub rate: f64,
}

/// Background animation data.
///
/// Holds three independent event tracks that the renderer can composite.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bga {
    /// Resource declarations (id → file path).
    pub resources: Vec<BgaResource>,
    /// Primary background animation events.
    pub events: Vec<BgaTimelineEvent>,
    /// Overlay layer events composited on top of the primary BGA.
    pub layer_events: Vec<BgaTimelineEvent>,
    /// Poor-performance (miss) animation events.
    pub poor_events: Vec<BgaTimelineEvent>,
}

/// A BGA resource (image or video file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgaResource {
    /// Unique identifier within the chart.
    pub id: u32,
    /// File path relative to the chart file's directory.
    pub path: PathBuf,
}

/// A BGA display event referencing a resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgaTimelineEvent {
    /// Tick position at which the resource becomes visible.
    pub tick: u64,
    /// Index into [`Bga::resources`].
    pub resource_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
