//! osu!-specific custom events and chart-level metadata.

use bmsrs_chart::CustomEvent;

/// An osu! game mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OsuMode {
    /// osu!standard.
    #[default]
    Osu,
    /// osu!taiko.
    Taiko,
    /// osu!catch.
    Catch,
    /// osu!mania.
    Mania,
}

/// Chart-level osu! metadata and difficulty settings.
///
/// Floating-point difficulty values are stored as scaled fixed-point
/// integers to keep `Eq` free of NaN conflicts; read them back through the
/// corresponding getters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OsuChartMeta {
    /// The `.osu` file format version.
    pub format_version: i32,
    /// The game mode.
    pub mode: OsuMode,
    /// The beatmap creator.
    pub creator: String,
    /// The song source.
    pub source: String,
    /// The search tags.
    pub tags: String,
    /// The original title (Unicode).
    pub title_unicode: String,
    /// The original artist (Unicode).
    pub artist_unicode: String,
    /// The osu! beatmap ID.
    pub beatmap_id: i32,
    /// The osu! beatmap set ID.
    pub beatmap_set_id: i32,
    /// Silent lead-in duration before the audio starts, in milliseconds.
    pub audio_lead_in_ms: u32,
    /// Song preview time in milliseconds; `-1` means unset.
    pub preview_time_ms: i32,
    /// HP drain rate, scaled by 10.
    pub hp_drain_rate_x10: u16,
    /// Overall difficulty, scaled by 10.
    pub overall_difficulty_x10: u16,
    /// Approach rate, scaled by 10.
    pub approach_rate_x10: u16,
    /// The mania key count (from `CircleSize`).
    pub key_count: u8,
    /// Slider multiplier, scaled by 100.
    pub slider_multiplier_x100: u32,
    /// Slider tick rate, scaled by 100.
    pub slider_tick_rate_x100: u32,
    /// Stack leniency, scaled by 1000.
    pub stack_leniency_x1000: u16,
}

impl OsuChartMeta {
    /// Return the HP drain rate.
    #[must_use]
    pub fn hp_drain_rate(&self) -> f32 {
        f32::from(self.hp_drain_rate_x10) / 10.0
    }

    /// Return the overall difficulty.
    #[must_use]
    pub fn overall_difficulty(&self) -> f32 {
        f32::from(self.overall_difficulty_x10) / 10.0
    }

    /// Return the approach rate.
    #[must_use]
    pub fn approach_rate(&self) -> f32 {
        f32::from(self.approach_rate_x10) / 10.0
    }

    /// Return the slider multiplier.
    #[must_use]
    pub fn slider_multiplier(&self) -> f64 {
        f64::from(self.slider_multiplier_x100) / 100.0
    }

    /// Return the slider tick rate.
    #[must_use]
    pub fn slider_tick_rate(&self) -> f64 {
        f64::from(self.slider_tick_rate_x100) / 100.0
    }

    /// Return the stack leniency.
    #[must_use]
    pub fn stack_leniency(&self) -> f32 {
        f32::from(self.stack_leniency_x1000) / 1000.0
    }
}

/// osu!-specific chart events.
///
/// The tick is stored uniformly by [`Event`](bmsrs_chart::Event); the
/// variants of this enum carry no tick of their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OsuCustomEvent {
    /// Chart-level metadata, generated once at tick 0.
    ChartMeta(OsuChartMeta),
    /// Time signature information from an uninherited timing point.
    Timing {
        /// Beats per measure.
        time_signature: u32,
        /// Whether the first bar line is omitted.
        omit_first_bar_line: bool,
    },
    /// Kiai toggle.
    Kiai(bool),
    /// A break interval; the event tick is the start, `end_tick` the end.
    Break {
        /// The break end tick.
        end_tick: u64,
    },
}

impl CustomEvent for OsuCustomEvent {}
