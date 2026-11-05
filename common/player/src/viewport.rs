//! Scroll-domain viewport projection.
//!
//! [`Viewport`] maps the player's timing and scroll state into screen-agnostic
//! scroll units: one beat of scroll distance and a normalized `0..=1` progress.
//! It deliberately does not know lane heights or pixel layouts; a renderer maps
//! scroll units to its own coordinates.

use std::ops::{Bound, RangeBounds};
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
    ///
    /// Convenience wrapper over [`visible_notes_in`](Self::visible_notes_in)
    /// with the default window `0.0..=max_scroll_delta_at`.
    pub fn visible_notes_at<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
    ) -> impl Iterator<Item = VisibleNote<'a, T, C>> + 'a {
        let max_delta = self.max_scroll_delta_at(player, time);
        // An empty range reproduces the old "no window, nothing visible"
        // behavior when cover/lift or SPEED collapses the lane.
        let range = if max_delta <= 0.0 {
            1.0..=0.0
        } else {
            0.0..=max_delta
        };
        self.visible_notes_in(player, time, range)
    }

    /// Return notes whose scroll delta from the judgment line falls in
    /// `range`, regardless of the lane direction.
    ///
    /// The delta is `scroll_position(note) - scroll_position(cursor)`: notes
    /// ahead of the judgment line have positive deltas, notes behind it
    /// negative ones. During reverse-scroll sections future notes approach
    /// from below, so pass a range with a negative lower bound (or `..` for
    /// everything) to see them.
    ///
    /// `progress` stays `delta / max_scroll_delta_at` regardless of `range`:
    /// the judgment line is always 0.0 and the lane top always 1.0.
    pub fn visible_notes_in<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
        range: impl RangeBounds<f64>,
    ) -> impl Iterator<Item = VisibleNote<'a, T, C>> + 'a {
        let cursor_tick = player.duration_to_tick_f64(time);
        let cursor_position = player.scroll_position_at_f64(cursor_tick);
        let max_delta = self.max_scroll_delta_at(player, time);
        let (lo, hi) = delta_bounds(&range);

        player.note_table().notes().iter().filter_map(move |point| {
            let note_position = player.scroll_position_at_f64(tick_to_f64(point.tick));
            let delta = note_position - cursor_position;
            if !contains_delta(lo, hi, delta) {
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
    ///
    /// Convenience wrapper over [`visible_long_notes_in`](Self::visible_long_notes_in)
    /// with the default window `0.0..=max_scroll_delta_at`.
    pub fn visible_long_notes_at<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
    ) -> impl Iterator<Item = VisibleLongNote<'a>> + 'a {
        let max_delta = self.max_scroll_delta_at(player, time);
        let range = if max_delta <= 0.0 {
            1.0..=0.0
        } else {
            0.0..=max_delta
        };
        self.visible_long_notes_in(player, time, range)
    }

    /// Return long notes whose body overlaps the delta `range`.
    ///
    /// A body overlaps when its end is at or above `lo` and its start is at or
    /// below `hi`; active holds with a negative start are included, matching
    /// the previous behavior.
    pub fn visible_long_notes_in<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
        range: impl RangeBounds<f64>,
    ) -> impl Iterator<Item = VisibleLongNote<'a>> + 'a {
        let cursor_tick = player.duration_to_tick_f64(time);
        let cursor_position = player.scroll_position_at_f64(cursor_tick);
        let (lo, hi) = delta_bounds(&range);

        player.long_notes().iter().filter_map(move |pair| {
            let start =
                player.scroll_position_at_f64(tick_to_f64(pair.start_tick)) - cursor_position;
            let end = player.scroll_position_at_f64(tick_to_f64(pair.end_tick)) - cursor_position;
            // The body may be reversed (start above end) inside a
            // reverse-scroll section; overlap is tested on the ordered span.
            let (body_lo, body_hi) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            if body_hi < lo || body_lo > hi {
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
    ///
    /// Convenience wrapper over [`visible_lines_in`](Self::visible_lines_in)
    /// with the default window `0.0..=max_scroll_delta_at`.
    pub fn visible_lines_at<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
    ) -> impl Iterator<Item = VisibleLine<'a, T, C>> + 'a {
        let max_delta = self.max_scroll_delta_at(player, time);
        let range = if max_delta <= 0.0 {
            1.0..=0.0
        } else {
            0.0..=max_delta
        };
        self.visible_lines_in(player, time, range)
    }

    /// Return bar lines, BPM changes, and STOP markers whose scroll delta
    /// falls in `range`.
    ///
    /// The delta semantics match [`visible_notes_in`](Self::visible_notes_in);
    /// during reverse-scroll sections markers approach from below, so pass a
    /// range with a negative lower bound to see them.
    pub fn visible_lines_in<'a, T: NoteExt, C: CustomEvent>(
        &'a self,
        player: &'a Player<T, C>,
        time: Duration,
        range: impl RangeBounds<f64>,
    ) -> impl Iterator<Item = VisibleLine<'a, T, C>> + 'a {
        let cursor_tick = player.duration_to_tick_f64(time);
        let cursor_position = player.scroll_position_at_f64(cursor_tick);
        let max_delta = self.max_scroll_delta_at(player, time);
        let (lo, hi) = delta_bounds(&range);

        player.events().iter().filter_map(move |event| {
            if !matches!(
                event.kind,
                EventKind::Bar | EventKind::Bpm { .. } | EventKind::Stop { .. }
            ) {
                return None;
            }
            let position =
                player.scroll_position_at_f64(tick_to_f64(event.tick())) - cursor_position;
            if !contains_delta(lo, hi, position) {
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
    ///
    /// This is the default window top used by the `visible_*_at` queries and
    /// the denominator of `progress`; build custom ranges symmetric to it by
    /// negating the value.
    #[must_use]
    pub fn max_scroll_delta_at<T: NoteExt, C: CustomEvent>(
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

/// Resolve a delta range into inclusive `(lo, hi)` bounds.
fn delta_bounds(range: &impl RangeBounds<f64>) -> (f64, f64) {
    let lo = match range.start_bound() {
        Bound::Included(v) => *v,
        Bound::Excluded(v) => f64::from_bits(v.to_bits().wrapping_add(1)),
        Bound::Unbounded => f64::NEG_INFINITY,
    };
    let hi = match range.end_bound() {
        Bound::Included(v) => *v,
        Bound::Excluded(v) => f64::from_bits(v.to_bits().wrapping_sub(1)),
        Bound::Unbounded => f64::INFINITY,
    };
    (lo, hi)
}

/// Return whether `delta` is inside the inclusive `(lo, hi)` bounds.
fn contains_delta(lo: f64, hi: f64, delta: f64) -> bool {
    delta >= lo && delta <= hi
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;

    use bmsrs_chart::{
        Chart, ChartData, ChartInfo, Event, EventKind, Lane, Note, NoteAudio, NoteKind, NoteRole,
        NoteSide, SongInfo,
    };

    const KEY1: NonZeroU8 = NonZeroU8::MIN;

    fn chart(events: Vec<Event>) -> Chart {
        let data = ChartData::new(120.0, 240, events, vec![])
            .unwrap()
            .with_judge_multiplier(Some(1.0))
            .with_life_multiplier(Some(1.0));
        Chart::new(SongInfo::default(), ChartInfo::default(), data).unwrap()
    }

    fn note(tick: u64) -> Event {
        Event::new(
            tick,
            EventKind::Note(Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Normal,
                role: NoteRole::Normal,
                audio: NoteAudio::none(),
                ext: (),
            }),
        )
    }

    #[test]
    fn future_note_is_visible() {
        let player = Player::new(chart(vec![Event::bar(0), note(240)]));
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
            Event::stop(240, Duration::from_millis(500)),
        ]));
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
            EventKind::Note(Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Long { duration: 200 },
                role: NoteRole::Normal,
                audio: NoteAudio::none(),
                ext: (),
            }),
        );
        let player = Player::new(chart(vec![
            long,
            Event::bar(240),
            Event::stop(240, Duration::from_millis(500)),
        ]));
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
        ]));
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
        let player = Player::new(chart(vec![Event::speed(0, 0.25), note(480)]));
        let viewport = Viewport::new(1.0, 1.0).with_constant(true);
        assert_eq!(
            viewport.visible_notes_at(&player, Duration::ZERO).count(),
            1
        );
    }

    /// Build a chart with a reverse-scroll section and three notes around it:
    /// rate flips to -0.5 at tick 480, so at T(700) the cursor scroll position
    /// is 1.5417 beats and every note sits below the judgment line.
    fn reverse_chart() -> Chart {
        chart(vec![
            Event::bar(0),
            Event::scroll(480, -0.5),
            note(240),
            note(720),
            note(960),
        ])
    }

    #[test]
    fn reverse_section_future_notes_hidden_by_default_window() {
        let player = Player::new(reverse_chart());
        let viewport = Viewport::new(1.0, 1.0);
        // At T(700) = 1.45833s every note (behind, at, and ahead of the
        // cursor) has a negative delta inside the reverse section.
        let visible = viewport.visible_notes_at(&player, Duration::from_secs_f64(1.45833));
        assert_eq!(visible.count(), 0);
    }

    #[test]
    fn reverse_section_notes_visible_with_unbounded_range() {
        let player = Player::new(reverse_chart());
        let viewport = Viewport::new(1.0, 1.0);
        let visible: Vec<_> = viewport
            .visible_notes_in(&player, Duration::from_secs_f64(1.45833), ..)
            .collect();
        assert_eq!(visible.len(), 3);
        assert!(
            visible.iter().all(|v| v.scroll_position < 0.0),
            "all three notes sit below the judgment line"
        );
    }

    #[test]
    fn unbounded_range_matches_manual_full_range() {
        let player = Player::new(reverse_chart());
        let viewport = Viewport::new(1.0, 1.0);
        let time = Duration::from_secs_f64(1.45833);
        let unbounded = viewport.visible_notes_in(&player, time, ..).count();
        let manual = viewport
            .visible_notes_in(&player, time, f64::NEG_INFINITY..=f64::INFINITY)
            .count();
        assert_eq!(unbounded, manual);
    }

    #[test]
    fn custom_delta_range_returns_below_line_notes() {
        let player = Player::new(reverse_chart());
        let viewport = Viewport::new(1.0, 1.0);
        let visible =
            viewport.visible_notes_in(&player, Duration::from_secs_f64(1.45833), -10.0..=0.0);
        assert_eq!(visible.count(), 3);
    }

    #[test]
    fn progress_stays_signed_over_default_window() {
        let player = Player::new(reverse_chart());
        let viewport = Viewport::new(1.0, 1.0);
        let below = viewport
            .visible_notes_in(&player, Duration::from_secs_f64(1.45833), ..)
            .find(|v| v.progress < 0.0)
            .expect("below-line notes have negative progress");
        // progress = delta / max_delta with max_delta = 4 beats; every delta
        // is within one mirrored window, so progress stays above -1.
        assert!(below.progress > -1.0);
    }

    #[test]
    fn max_scroll_delta_at_matches_lane_formula() {
        let player = Player::new(reverse_chart());
        let viewport = Viewport::new(1.0, 1.0);
        // hispeed 1, visible fraction 1, no SPEED events: one measure spans
        // the lane, so the top is exactly 4 beats.
        let top = viewport.max_scroll_delta_at(&player, Duration::ZERO);
        assert!((top - 4.0).abs() < 1e-9);
    }

    #[test]
    fn reverse_section_lines_rise_from_below() {
        let player = Player::new(chart(vec![
            Event::bar(0),
            Event::bar(480),
            Event::scroll(480, -1.0),
            Event::bar(960),
        ]));
        let viewport = Viewport::new(1.0, 1.0);
        // At tick 840 (1.75 s at 120 BPM) the cursor scroll position is 0.5
        // beats: bar 0 (pos 0) and bar 960 (pos 0) are 0.5 beats below the
        // line, bar 480 (pos 2) is 1.5 beats above it.
        let below = viewport.visible_lines_in(&player, Duration::from_secs_f64(1.75), -10.0..=0.0);
        assert_eq!(below.count(), 2);
        let above = viewport.visible_lines_at(&player, Duration::from_secs_f64(1.75));
        assert_eq!(above.count(), 1);
    }

    #[test]
    fn reverse_section_long_note_body_crosses_lower_window() {
        let long = Event::new(
            480,
            EventKind::Note(Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Long { duration: 480 },
                role: NoteRole::Normal,
                audio: NoteAudio::none(),
                ext: (),
            }),
        );
        let player = Player::new(chart(vec![Event::bar(0), Event::scroll(480, -1.0), long]));
        let viewport = Viewport::new(1.0, 1.0);
        // At tick 600 (1.25 s at 120 BPM) the LN body spans +0.5 down to
        // -1.5 (reversed body): it crosses the judgment line, so both windows
        // contain it.
        let time = Duration::from_secs_f64(1.25);
        let lower = viewport.visible_long_notes_in(&player, time, -10.0..=0.0);
        assert_eq!(lower.count(), 1);
        let default = viewport.visible_long_notes_at(&player, time);
        assert_eq!(default.count(), 1);
    }
}
