//! Stable note index and descriptive judgment-point metadata.
//!
//! The table is built by [`Player`](crate::Player) from the chart event stream.
//! It owns no judgment policy: a [`NotePoint`] says what the source format
//! describes (tap, long note, mine, invisible, release marker), and the caller
//! decides how its Ln/Cn/Hcn policy judges it.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Bound, RangeBounds};

use bmsrs_chart::{
    CustomEvent, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint, Note, NoteAudio,
    NoteExt, NoteKind, NoteRole, NoteSide,
};

/// Stable identifier assigned by [`NoteTable`] in event order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(u32);

impl NoteId {
    /// Create a note id from a raw index.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the raw index value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for NoteId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Descriptive judgment-point metadata for one note event.
///
/// `judgeable` is deliberately absent. The same `NoteKind`/`NoteRole` pair can
/// mean different things under different long-note modes, so the caller applies
/// its own policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotePoint {
    /// Stable note id.
    pub id: NoteId,
    /// Player side.
    pub side: NoteSide,
    /// Key, scratch, or pedal.
    pub lane: Lane,
    /// Tick position.
    pub tick: u64,
    /// Note kind.
    pub kind: NoteKind,
    /// Note role.
    pub role: NoteRole,
    /// Per-note long-note type override, if the source format provided one.
    pub ln_type_hint: Option<LnTypeHint>,
    /// Per-note long-note judgment override, if the source format provided one.
    pub ln_judge_hint: Option<LnJudgeHint>,
    /// Per-note long-note life override, if the source format provided one.
    pub ln_life_hint: Option<LnLifeHint>,
    /// Audio layers attached to the note.
    pub audio: NoteAudio,
    /// Index into the chart's sorted event vector.
    pub event_index: usize,
}

/// A note point together with its source event.
#[derive(Clone, Copy, Debug)]
pub struct NoteRef<'a, T: NoteExt, C: CustomEvent> {
    /// Descriptive note point.
    point: &'a NotePoint,
    /// Source event.
    event: &'a Event<T, C>,
}

impl<'a, T: NoteExt, C: CustomEvent> NoteRef<'a, T, C> {
    /// Return the descriptive note point.
    #[must_use]
    pub const fn point(&self) -> &'a NotePoint {
        self.point
    }

    /// Return the source event.
    #[must_use]
    pub const fn event(&self) -> &'a Event<T, C> {
        self.event
    }

    /// Return the stable note id.
    #[must_use]
    pub const fn id(&self) -> NoteId {
        self.point.id
    }
}

/// Long-note start and the tick where its hold ends.
///
/// `end_note` is present when the source format has a distinct release marker at
/// the end tick (for example BMSON `up: true`). BMS long notes only carry a
/// duration, so `end_note` is `None` for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongNotePair {
    /// Start note id.
    pub start: NoteId,
    /// Player side.
    pub side: NoteSide,
    /// Key, scratch, or pedal.
    pub lane: Lane,
    /// Start tick.
    pub start_tick: u64,
    /// Exclusive end tick.
    pub end_tick: u64,
    /// Optional release marker at `end_tick`.
    pub end_note: Option<NoteId>,
}

/// Per-lane note id list.
#[derive(Clone, Debug, Default)]
struct LaneNotes {
    /// Note ids for this lane, in event order.
    ids: Vec<NoteId>,
}

/// Stable note index built from the sorted event stream.
pub struct NoteTable {
    /// All note points, in event order.
    points: Vec<NotePoint>,
    /// Note ids grouped by side and lane.
    by_lane: BTreeMap<(NoteSide, Lane), LaneNotes>,
    /// Long-note starts and their end ticks.
    long_notes: Vec<LongNotePair>,
}

impl NoteTable {
    /// Build the note table from a sorted event stream.
    #[expect(
        clippy::expect_used,
        reason = "a chart cannot contain more than u32::MAX note events"
    )]
    pub(crate) fn build<T: NoteExt, C: CustomEvent>(events: &[Event<T, C>]) -> Self {
        let mut points = Vec::new();
        let mut by_lane: BTreeMap<(NoteSide, Lane), LaneNotes> = BTreeMap::new();

        for (event_index, event) in events.iter().enumerate() {
            let EventKind::Note(Note {
                side,
                lane,
                kind,
                role,
                audio,
                ext,
            }) = &event.kind
            else {
                continue;
            };

            let id = NoteId::new(u32::try_from(points.len()).expect("note count fits u32"));
            let point = NotePoint {
                id,
                side: *side,
                lane: *lane,
                tick: event.tick(),
                kind: *kind,
                role: *role,
                ln_type_hint: ext.ln_type_hint(),
                ln_judge_hint: ext.ln_judge_hint(),
                ln_life_hint: ext.ln_life_hint(),
                audio: audio.clone(),
                event_index,
            };
            points.push(point);
            by_lane.entry((*side, *lane)).or_default().ids.push(id);
        }

        let release_at: BTreeMap<(NoteSide, Lane, u64), NoteId> = points
            .iter()
            .filter(|point| point.role == NoteRole::Release)
            .map(|point| ((point.side, point.lane, point.tick), point.id))
            .collect();

        let mut long_notes = Vec::new();
        for point in &points {
            let NoteKind::Long { duration } = point.kind else {
                continue;
            };
            let end_tick = point.tick.saturating_add(duration);
            long_notes.push(LongNotePair {
                start: point.id,
                side: point.side,
                lane: point.lane,
                start_tick: point.tick,
                end_tick,
                end_note: release_at.get(&(point.side, point.lane, end_tick)).copied(),
            });
        }

        Self {
            points,
            by_lane,
            long_notes,
        }
    }

    /// Return every note point in event order.
    #[must_use]
    pub fn notes(&self) -> &[NotePoint] {
        &self.points
    }

    /// Return one note point by id.
    #[must_use]
    pub fn note(&self, id: NoteId) -> Option<&NotePoint> {
        let index = usize::try_from(id.get()).ok()?;
        self.points.get(index)
    }

    /// Return note ids for one side/lane pair.
    #[must_use]
    pub fn notes_for_lane(&self, side: NoteSide, lane: Lane) -> &[NoteId] {
        self.by_lane
            .get(&(side, lane))
            .map_or(&[], |lane_notes| lane_notes.ids.as_slice())
    }

    /// Return every long-note pair.
    #[must_use]
    pub fn long_notes(&self) -> &[LongNotePair] {
        &self.long_notes
    }

    /// Return note points in `range`, sorted by tick.
    pub fn notes_in_range(&self, range: impl RangeBounds<u64>) -> impl Iterator<Item = &NotePoint> {
        let (start, end) = self.range_indices(range);
        let points: &[NotePoint] = self.points.get(start..end).map_or(&[], |slice| slice);
        points.iter()
    }

    /// Return the note ref for `point` in the chart's event vector.
    #[expect(
        clippy::expect_used,
        reason = "event_index is produced from the same event vector"
    )]
    pub(crate) fn note_ref<'a, T: NoteExt, C: CustomEvent>(
        point: &'a NotePoint,
        events: &'a [Event<T, C>],
    ) -> NoteRef<'a, T, C> {
        let event = events
            .get(point.event_index)
            .expect("note event index must be in range");
        NoteRef { point, event }
    }

    /// Map a range to `(start, end)` indices in `self.points`.
    fn range_indices(&self, range: impl RangeBounds<u64>) -> (usize, usize) {
        let start = match range.start_bound() {
            Bound::Included(tick) => self.points.partition_point(|point| point.tick < *tick),
            Bound::Excluded(tick) => self.points.partition_point(|point| point.tick <= *tick),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(tick) => self.points.partition_point(|point| point.tick <= *tick),
            Bound::Excluded(tick) => self.points.partition_point(|point| point.tick < *tick),
            Bound::Unbounded => self.points.len(),
        };
        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;

    use bmsrs_chart::{NoCustomEvent, Note, NoteAudio};

    const KEY1: NonZeroU8 = NonZeroU8::MIN;

    fn note(
        tick: u64,
        side: NoteSide,
        lane: Lane,
        kind: NoteKind,
        role: NoteRole,
    ) -> Event<(), NoCustomEvent> {
        Event::new(
            tick,
            EventKind::Note(Note {
                side,
                lane,
                kind,
                role,
                audio: NoteAudio::none(),
                ext: (),
            }),
        )
    }

    #[test]
    fn note_ids_are_assigned_in_event_order() {
        let events = vec![
            note(
                0,
                NoteSide::P1,
                Lane::Key(KEY1),
                NoteKind::Normal,
                NoteRole::Normal,
            ),
            Event::bar(1),
            note(
                2,
                NoteSide::P1,
                Lane::Key(KEY1),
                NoteKind::Normal,
                NoteRole::Normal,
            ),
        ];
        let table = NoteTable::build(&events);
        assert_eq!(table.notes().len(), 2);
        assert_eq!(table.notes()[0].id, NoteId::new(0));
        assert_eq!(table.notes()[1].id, NoteId::new(1));
        assert_eq!(table.notes()[1].event_index, 2);
    }

    #[test]
    fn notes_for_lane_groups_by_side_and_lane() {
        let events = vec![
            note(
                0,
                NoteSide::P1,
                Lane::Key(KEY1),
                NoteKind::Normal,
                NoteRole::Normal,
            ),
            note(
                0,
                NoteSide::P2,
                Lane::Key(KEY1),
                NoteKind::Normal,
                NoteRole::Normal,
            ),
        ];
        let table = NoteTable::build(&events);
        assert_eq!(table.notes_for_lane(NoteSide::P1, Lane::Key(KEY1)).len(), 1);
        assert_eq!(table.notes_for_lane(NoteSide::P2, Lane::Key(KEY1)).len(), 1);
        assert_eq!(
            table
                .notes_for_lane(NoteSide::P1, Lane::Scratch(KEY1))
                .len(),
            0
        );
    }

    #[test]
    fn long_note_pair_records_end_tick() {
        let events = vec![note(
            240,
            NoteSide::P1,
            Lane::Key(KEY1),
            NoteKind::Long { duration: 240 },
            NoteRole::Normal,
        )];
        let table = NoteTable::build(&events);
        assert_eq!(table.long_notes().len(), 1);
        assert_eq!(table.long_notes()[0].start_tick, 240);
        assert_eq!(table.long_notes()[0].end_tick, 480);
        assert!(table.long_notes()[0].end_note.is_none());
    }

    #[test]
    fn long_note_pair_records_release_end_note() {
        let events = vec![
            note(
                240,
                NoteSide::P1,
                Lane::Key(KEY1),
                NoteKind::Long { duration: 240 },
                NoteRole::Normal,
            ),
            note(
                480,
                NoteSide::P1,
                Lane::Key(KEY1),
                NoteKind::Normal,
                NoteRole::Release,
            ),
        ];
        let table = NoteTable::build(&events);
        assert_eq!(table.long_notes()[0].end_note, Some(NoteId::new(1)));
    }

    #[test]
    fn release_marker_is_preserved() {
        let events = vec![note(
            480,
            NoteSide::P1,
            Lane::Key(KEY1),
            NoteKind::Normal,
            NoteRole::Release,
        )];
        let table = NoteTable::build(&events);
        assert_eq!(table.notes()[0].role, NoteRole::Release);
    }

    #[test]
    fn notes_in_range_includes_max_tick() {
        let events = vec![note(
            u64::MAX,
            NoteSide::P1,
            Lane::Key(KEY1),
            NoteKind::Normal,
            NoteRole::Normal,
        )];
        let table = NoteTable::build(&events);
        assert_eq!(table.notes_in_range(..).count(), 1);
        assert_eq!(table.notes_in_range(..=u64::MAX).count(), 1);
    }
}
