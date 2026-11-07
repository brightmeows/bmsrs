//! Timing-domain conversion steps: BPM, STOP, SCROLL, SPEED, and bar lines.

use std::collections::BTreeMap;
use std::time::Duration;

use bmsrs_bms_model::BpmValue;
use bmsrs_bms_values::BmsChannel as RawChannel;
use bmsrs_chart::{BpmChange, BpmLookup, Event, StopEvent};

use super::{BmsConverter, BmsEvent, ProcessWarning, ProcessWarningKind};
use crate::position::MeasureTable;

impl BmsConverter<'_> {
    /// Build BPM change events for the timing track.
    ///
    /// Sort by `tick` first, then deduplicate the ordered sequence: adjacent
    /// changes with the same value keep the first. The order cannot be reversed:
    /// `messages.bpm_changes` accumulates by (measure, channel) rather than by
    /// tick, so deduplicating before sorting would merge equal-value changes that
    /// are adjacent in the vector but not adjacent in time.
    pub fn build_bpm_changes(&mut self) -> Vec<BpmChange> {
        let mut changes: Vec<BpmChange> = self
            .bms
            .messages
            .bpm_changes
            .iter()
            .filter_map(|bc| {
                let tick = self.table.position_to_tick(bc.position);
                let span = self.message_span(
                    bc.position.measure(),
                    &[RawChannel::BpmChange, RawChannel::ExtendedBpm],
                );
                let bpm = self.resolve_bpm(bc.value, tick, span);
                if BpmChange::is_valid(bpm) {
                    Some(BpmChange { tick, bpm })
                } else {
                    self.warnings.push(ProcessWarning::new(
                        span,
                        ProcessWarningKind::InvalidBpmChangeIgnored { tick, bpm },
                    ));
                    None
                }
            })
            .collect();
        // Deduplication must happen after sorting by tick: two equal-value
        // changes in the same channel are adjacent in the vector (chunked by
        // channel). If a change from another channel sits between them,
        // deduplicating before sorting would merge them incorrectly.
        changes.sort_by_key(|c| c.tick);
        changes.dedup_by(|a, b| (a.bpm - b.bpm).abs() < f64::EPSILON);
        changes
    }

    /// Resolve a [`BpmValue`] into a concrete BPM, using the BPM definition table
    /// for references.
    fn resolve_bpm(
        &mut self,
        value: BpmValue,
        tick: u64,
        span: Option<bmsrs_pipeline::SourceSpan>,
    ) -> f64 {
        let base = self.bms.gameplay.base.unwrap_or_default();
        match value {
            BpmValue::Absolute(bpm) => bpm,
            BpmValue::Reference(id) => {
                if let Some(bpm) = self.bms.timing.bpm_defs.get(&id).copied() {
                    bpm
                } else {
                    if let Some(key) = id.to_index_in(base) {
                        self.warnings.push(ProcessWarning::new(
                            span,
                            ProcessWarningKind::MissingBpmDefinition { key, tick },
                        ));
                    }
                    120.0
                }
            }
        }
    }

    /// Build stop events from `#STOPxx` definitions (channel `09`).
    ///
    /// Negative STOP values are clamped to `0` (skipped) with a warning.
    /// Angolmois ignores negative definitions and bms-rs rejects them at the
    /// parse layer; beatoraja instead plays the absolute value, a deliberate
    /// difference here.
    pub fn build_stops_from_defs(&mut self, bpm_lookup: &BpmLookup<'_>) -> Vec<StopEvent> {
        let base = self.bms.gameplay.base.unwrap_or_default();
        self.bms
            .messages
            .stop_events
            .iter()
            .filter_map(|se| {
                let tick = self.table.position_to_tick(se.position);
                let span = self.message_span(se.position.measure(), &[RawChannel::Stop]);
                if let Some(&raw) = self.bms.timing.stop_defs.get(&se.stop_id) {
                    // The raw value counts 1/192 notes; the pause length in
                    // seconds depends on the BPM in effect at the stop tick.
                    let secs = (raw / 192.0 * 4.0) * 60.0 / bpm_lookup.bpm_at_tick(tick).abs();
                    if secs < 0.0 {
                        self.warnings.push(ProcessWarning::new(
                            span,
                            ProcessWarningKind::StopDurationClipped {
                                tick,
                                original: secs,
                            },
                        ));
                    }
                    Some(StopEvent {
                        tick,
                        duration: Duration::try_from_secs_f64(secs.max(0.0))
                            .unwrap_or(Duration::ZERO),
                    })
                } else {
                    if let Some(key) = se.stop_id.to_index_in(base) {
                        self.warnings.push(ProcessWarning::new(
                            span,
                            ProcessWarningKind::MissingStopDefinition { key, tick },
                        ));
                    }
                    None
                }
            })
            .collect()
    }

    /// Build stop events from the `#STP` header command (duration in
    /// milliseconds).
    ///
    /// Entries at the same position accumulate per the specification: the
    /// example pair `#STP 001.500 500` and `#STP 001.500 4000` stops scrolling
    /// for 4500 ms in total. Entries are grouped by the resolved tick, so
    /// distinct positions that quantize to the same tick also merge into one
    /// stop. Non-positive durations are skipped; negative entries warn with
    /// [`ProcessWarningKind::StopDurationClipped`], mirroring the `#STOP`
    /// channel policy above. beatoraja does not implement `#STP` at all.
    pub fn build_stops_from_stp(&mut self) -> Vec<StopEvent> {
        let mut totals_ms: BTreeMap<u64, f64> = BTreeMap::new();
        for stp in &self.bms.messages.stp_events {
            let tick = self.table.position_to_tick(stp.position);
            if stp.duration_ms < 0.0 {
                self.warnings.push(ProcessWarning::new(
                    None,
                    ProcessWarningKind::StopDurationClipped {
                        tick,
                        original: stp.duration_ms / 1000.0,
                    },
                ));
                continue;
            }
            if stp.duration_ms == 0.0 {
                continue;
            }
            *totals_ms.entry(tick).or_default() += stp.duration_ms;
        }
        totals_ms
            .into_iter()
            .map(|(tick, total_ms)| StopEvent {
                tick,
                duration: Duration::try_from_secs_f64(total_ms / 1000.0).unwrap_or(Duration::ZERO),
            })
            .collect()
    }

    /// Build BPM events from the already-normalized BPM change list.
    ///
    /// The list is produced by [`build_bpm_changes`](Self::build_bpm_changes),
    /// so invalid values have already been filtered and warned about. Emitting
    /// from the same list keeps the event stream and the derived timing track
    /// identical and avoids resolving each BPM reference twice.
    pub fn collect_bpm_events(&mut self, changes: &[BpmChange]) {
        for change in changes {
            self.events.push(Event::bpm(change.tick, change.bpm));
        }
    }

    /// Emit stop events from the already-built stop event slice.
    pub fn collect_stop_events(&mut self, stops: &[StopEvent]) {
        for se in stops {
            self.events.push(Event::stop(se.tick, se.duration));
        }
    }

    /// Build SCROLL change events.
    pub fn collect_scroll_events(&mut self) {
        for se in &self.bms.messages.scroll_events {
            if let Some(&rate) = self.bms.timing.scroll_defs.get(&se.scroll_id) {
                self.events.push(Event::scroll(
                    self.table.position_to_tick(se.position),
                    rate,
                ));
            }
        }
    }

    /// Build SPEED (visual note spacing) keyframe events.
    pub fn collect_speed_events(&mut self) {
        for se in &self.bms.messages.speed_events {
            if let Some(&rate) = self.bms.timing.speed_defs.get(&se.speed_id) {
                self.events
                    .push(Event::speed(self.table.position_to_tick(se.position), rate));
            }
        }
    }
}

/// Build aligned bar events from the precomputed measure tick table.
///
/// Supports variable time signatures (`#xxx02`): each measure's barline sits at
/// its actual start tick instead of a forced equal-step 4/4 spacing.
pub fn build_bar_events(table: &MeasureTable) -> Vec<BmsEvent> {
    table
        .bar_ticks()
        .iter()
        .map(|&tick| Event::bar(tick))
        .collect()
}
