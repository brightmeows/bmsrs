//! Scroll-domain viewport projection.
//!
//! [`Viewport`] maps the player's timing and scroll state into screen-agnostic
//! scroll units: one beat of scroll distance and a normalized `0..=1` progress.
//! It deliberately does not know lane heights or pixel layouts; a renderer maps
//! scroll units to its own coordinates.

use std::time::Duration;

use bmsrs_chart::{CustomEvent, Event, EventKind, NoteExt};

use crate::Player;
use crate::note_table::{LongNotePair, NoteRef};

/// One visible note and its scroll-domain position.
#[derive(Clone, Copy, Debug)]
pub struct VisibleNote<'a, T: NoteExt, C: CustomEvent> {
    /// Note reference.
    pub note: NoteRef<'a, T, C>,
    /// Scroll distance from the judgment line, in beats.
    pub scroll_position: f64,
    /// Normalized progress from the judgment line (`0.0`) to the lane top
    /// (`1.0`).
    pub progress: f64,
}

/// A long note whose body overlaps the viewport.
#[derive(Clone, Copy, Debug)]
pub struct VisibleLongNote<'a> {
    /// Long-note pair.
    pub pair: &'a LongNotePair,
    /// Start scroll position in beats (may be negative for an active hold).
    pub start_scroll_position: f64,
    /// End scroll position in beats.
    pub end_scroll_position: f64,
}

/// A visible bar line, BPM change, or STOP marker.
#[derive(Clone, Copy, Debug)]
pub struct VisibleLine<'a, T: NoteExt, C: CustomEvent> {
    /// Source event.
    pub event: &'a Event<T, C>,
    /// Scroll distance from the judgment line, in beats.
    pub scroll_position: f64,
    /// Normalized progress from the judgment line to the lane top.
    pub progress: f64,
}

/// Scroll-domain viewport configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// High-speed multiplier.
    hispeed: f64,
    /// Visible lane fraction after cover/lift.
    visible_fraction: f64,
    /// Whether CONSTANT mode forces SPEED to 1.0.
    constant: bool,
}

impl Viewport {
    /// Create a viewport.
    ///
    /// `hispeed` is the player's high-speed multiplier; `visible_fraction` is
    /// the visible portion of the lane after cover/lift (0.0 to 1.0).
    #[must_use]
    pub fn new(hispeed: f64, visible_fraction: f64) -> Self {
        Self {
            hispeed: if hispeed.is_finite() && hispeed > 0.0 {
                hispeed
            } else {
                1.0
            },
            visible_fraction: if visible_fraction.is_finite() {
                visible_fraction.clamp(0.0, 1.0)
            } else {
                1.0
            },
            constant: false,
        }
    }

    /// Enable or disable CONSTANT mode (SPEED forced to 1.0).
    #[must_use]
    pub const fn with_constant(mut self, constant: bool) -> Self {
        self.constant = constant;
        self
    }

    /// Return the configured high-speed multiplier.
    #[must_use]
    pub const fn hispeed(&self) -> f64 {
        self.hispeed
    }

    /// Return the configured visible lane fraction.
    #[must_use]
    pub const fn visible_fraction(&self) -> f64 {
        self.visible_fraction
    }

    /// Return whether CONSTANT mode is enabled.
    #[must_use]
    pub const fn constant(&self) -> bool {
        self.constant
    }

    /// Return visible notes at `time`.
    pub fn visible_notes_at<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
    ) -> impl Iterator<Item = VisibleNote<'a, T, C>> + 'a {
        let cursor_tick = player.time_to_tick_f64(time);
        let cursor_position = player.scroll_position_at_f64(cursor_tick);
        let max_delta = self.max_scroll_delta(player, time);

        player.note_table().notes().iter().filter_map(move |point| {
            if max_delta <= 0.0 {
                return None;
            }
            let note_position = player.scroll_position_at_f64(tick_to_f64(point.tick));
            let delta = note_position - cursor_position;
            if !(0.0..=max_delta).contains(&delta) {
                return None;
            }
            let note = player.note(point.id)?;
            Some(VisibleNote {
                note,
                scroll_position: delta,
                progress: delta / max_delta,
            })
        })
    }

    /// Return long notes whose body overlaps the viewport.
    pub fn visible_long_notes_at<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
    ) -> impl Iterator<Item = VisibleLongNote<'a>> + 'a {
        let cursor_tick = player.time_to_tick_f64(time);
        let cursor_position = player.scroll_position_at_f64(cursor_tick);
        let max_delta = self.max_scroll_delta(player, time);

        player.long_notes().iter().filter_map(move |pair| {
            if max_delta <= 0.0 {
                return None;
            }
            let start =
                player.scroll_position_at_f64(tick_to_f64(pair.start_tick)) - cursor_position;
            let end = player.scroll_position_at_f64(tick_to_f64(pair.end_tick)) - cursor_position;
            if end < 0.0 || start > max_delta {
                return None;
            }
            Some(VisibleLongNote {
                pair,
                start_scroll_position: start,
                end_scroll_position: end,
            })
        })
    }

    /// Return visible bar lines, BPM changes, and STOP markers at `time`.
    pub fn visible_lines_at<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
    ) -> impl Iterator<Item = VisibleLine<'a, T, C>> + 'a {
        let cursor_tick = player.time_to_tick_f64(time);
        let cursor_position = player.scroll_position_at_f64(cursor_tick);
        let max_delta = self.max_scroll_delta(player, time);

        player.chart().data.events.iter().filter_map(move |event| {
            if max_delta <= 0.0 {
                return None;
            }
            if !matches!(
                event.kind,
                EventKind::Bar | EventKind::Bpm { .. } | EventKind::Stop { .. }
            ) {
                return None;
            }
            let position =
                player.scroll_position_at_f64(tick_to_f64(event.tick())) - cursor_position;
            if !(0.0..=max_delta).contains(&position) {
                return None;
            }
            Some(VisibleLine {
                event,
                scroll_position: position,
                progress: position / max_delta,
            })
        })
    }

    /// Return the scroll distance covered by the visible lane, in beats.
    fn max_scroll_delta<T: NoteExt, C: CustomEvent>(
        &self,
        player: &Player<T, C>,
        time: Duration,
    ) -> f64 {
        if self.visible_fraction <= 0.0 {
            return 0.0;
        }
        let speed = if self.constant {
            1.0
        } else {
            player.spacing_at_time(time)
        };
        if !speed.is_finite() || speed <= 0.0 {
            return 0.0;
        }
        // One measure (four beats) spans the full lane at hispeed 1 and SPEED 1.
        4.0 * self.visible_fraction / (self.hispeed * speed)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "tick values fit in f64 mantissa for practical chart lengths"
)]
/// Convert an integer tick to `f64` for continuous scroll math.
const fn tick_to_f64(tick: u64) -> f64 {
    tick as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;

    use bmsrs_chart::{
        Chart, ChartData, ChartInfo, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint,
        NoteAudio, NoteKind, NoteRole, NoteSide, SongInfo,
    };

    const KEY1: NonZeroU8 = NonZeroU8::MIN;

    fn chart(events: Vec<Event>) -> Chart {
        Chart {
            song: SongInfo::default(),
            chart: ChartInfo::default(),
            data: ChartData {
                init_bpm: 120.0,
                resolution: 240,
                judge_multiplier: Some(1.0),
                life_multiplier: Some(1.0),
                ln_type_hint: LnTypeHint::default(),
                ln_judge_hint: LnJudgeHint::default(),
                ln_life_hint: LnLifeHint::default(),
                judge_deltas: None,
                life_deltas: None,
                events,
                audio_assets: vec![],
            },
        }
    }

    fn note(tick: u64) -> Event {
        Event::new(
            tick,
            EventKind::Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Normal,
                role: NoteRole::Normal,
                audio: NoteAudio::none(),
                ext: (),
            },
        )
    }

    #[test]
    fn future_note_is_visible() {
        let player = Player::new(chart(vec![Event::bar(0), note(240)])).unwrap();
        let viewport = Viewport::new(1.0, 1.0);
        let visible: Vec<_> = viewport.visible_notes_at(&player, Duration::ZERO).collect();
        assert_eq!(visible.len(), 1);
        assert!(visible[0].progress > 0.0);
    }

    #[test]
    fn note_after_stop_stays_visible_while_frozen() {
        let player = Player::new(chart(vec![
            Event::bar(240),
            note(300),
            Event::stop(240, 240),
        ]))
        .unwrap();
        let viewport = Viewport::new(1.0, 1.0);
        assert_eq!(
            viewport
                .visible_notes_at(&player, Duration::from_secs_f64(0.6))
                .count(),
            1
        );
    }

    #[test]
    fn active_long_note_is_visible() {
        let long = Event::new(
            200,
            EventKind::Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Long { duration: 200 },
                role: NoteRole::Normal,
                audio: NoteAudio::none(),
                ext: (),
            },
        );
        let player =
            Player::new(chart(vec![long, Event::bar(240), Event::stop(240, 240)])).unwrap();
        let viewport = Viewport::new(1.0, 1.0);
        assert_eq!(
            viewport
                .visible_long_notes_at(&player, Duration::from_secs_f64(0.6))
                .count(),
            1
        );
    }

    #[test]
    fn speed_uses_time_axis() {
        let player = Player::new(chart(vec![
            Event::speed(0, 1.0),
            Event::speed(480, 2.0),
            note(480),
        ]))
        .unwrap();
        let viewport = Viewport::new(1.0, 1.0);
        // At tick 240 (0.5s), SPEED should be 1.0, so 480 ticks is two beats
        // away and still inside the four-beat viewport.
        assert_eq!(
            viewport
                .visible_notes_at(&player, Duration::from_secs_f64(0.5))
                .count(),
            1
        );
    }

    #[test]
    fn constant_mode_ignores_speed() {
        let player = Player::new(chart(vec![Event::speed(0, 0.25), note(480)])).unwrap();
        let viewport = Viewport::new(1.0, 1.0).with_constant(true);
        assert_eq!(
            viewport.visible_notes_at(&player, Duration::ZERO).count(),
            1
        );
    }
}
