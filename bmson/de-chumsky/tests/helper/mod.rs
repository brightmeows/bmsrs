//! Shared helper functions for integration tests.

/// Minimal valid v2.0.0 BMSON JSON string.
#[must_use]
pub const fn minimal_v2_json() -> &'static str {
    r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]}}"#
}

/// Minimal valid v1.0.0 BMSON JSON string.
#[must_use]
pub const fn minimal_v1_json() -> &'static str {
    r#"{"version":"1.0.0","info":{"title":"T","artist":"A","genre":"G","init_bpm":140.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#
}

/// Minimal valid v0.2.1 BMSON JSON string.
#[must_use]
pub const fn minimal_v0_json() -> &'static str {
    r#"{"info":{"title":"T","artist":"A","genre":"G","initBPM":140.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#
}
