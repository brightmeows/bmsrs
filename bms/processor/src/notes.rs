//! Note-domain conversion steps: playable notes, BGM, and long-note pairing.

use std::collections::{BTreeMap, HashMap, HashSet};

use bmsrs_bms_model::KeyType;
use bmsrs_bms_values::{LnMode, WavIndex};
use bmsrs_chart::{
    AudioIndex, Damage, Event, EventKind, LnTypeHint, Note, NoteAudio, NoteKind, NoteRole,
};

use crate::layout::{BmsLayout, ChannelPos};
use crate::long_note::{LnPairingResult, PairedLn, pair_lnobj, pair_lntype1, pair_lntype2};

use super::{BmsConverter, ProcessWarning, ProcessWarningKind};

impl BmsConverter<'_> {
    /// Determine the LN mode and pair long notes. Returns the pairing result and
    /// the consumed note indices.
    pub fn pair_long_notes(&self) -> (LnPairingResult, HashSet<usize>) {
        if let Some(ln_obj) = self.bms.gameplay.ln_obj {
            let (paired, consumed) = pair_lnobj(&self.bms.messages.note_events, ln_obj, self.table);
            return (
                LnPairingResult {
                    paired,
                    unpaired_starts: Vec::new(),
                },
                consumed,
            );
        }
        if self.bms.gameplay.ln_type == Some(bmsrs_bms_values::LnType::Type2) {
            return (
                pair_lntype2(&self.bms.messages.long_note_events, self.table),
                HashSet::new(),
            );
        }
        (
            pair_lntype1(&self.bms.messages.long_note_events, self.table),
            HashSet::new(),
        )
    }

    /// Derive the chart-level long-note type hint from the `#LNMODE` header.
    ///
    /// `LnMode{Ln,Cn,Hcn}` and `LnTypeHint{Ln,Cn,Hcn}` are 1:1 isomorphic enums.
    /// No `#LNMODE` maps to `None`: the unspecified state travels to the
    /// caller, matching beatoraja keeping the LN mode undefined.
    pub const fn ln_type_hint(&self) -> Option<LnTypeHint> {
        match self.bms.gameplay.ln_mode {
            Some(LnMode::Ln) => Some(LnTypeHint::Ln),
            Some(LnMode::Cn) => Some(LnTypeHint::Cn),
            Some(LnMode::Hcn) => Some(LnTypeHint::Hcn),
            None => None,
        }
    }

    /// Collect all playable notes into the event vector.
    ///
    /// Slot conflicts resolve in jbms-parser's channel processing order:
    /// visible notes (file order) come first, ch51-69 notes treated as
    /// visible under `#LNOBJ` follow, then paired long notes (start plus end
    /// marker), then mines. A later note or long-note start overwrites the
    /// same-position slot with a warning; an end marker overwrites silently
    /// (ch51-69) or with that warning (`#LNOBJ`); a mine never overwrites an
    /// occupied slot and is dropped instead, matching beatoraja.
    #[expect(
        clippy::too_many_lines,
        reason = "normal/invisible/LNOBJ/long-note/mine sources share one matching logic; splitting would fragment it"
    )]
    pub fn collect_notes<L: BmsLayout>(
        &mut self,
        wav_map: &BTreeMap<WavIndex, AudioIndex>,
        paired_lns: &[PairedLn],
        consumed: &HashSet<usize>,
        unpaired_starts: &[(u8, u8, u64)],
    ) {
        let base = self.bms.gameplay.base.unwrap_or_default();
        // The warnings vector is passed per call so the covered-note branch
        // inside the loop can push its own warning while iterating.
        let check_wav = |warnings: &mut Vec<ProcessWarning>,
                         wav_id: &WavIndex,
                         player: u8,
                         lane: u8,
                         tick: u64|
         -> Option<AudioIndex> {
            let audio = wav_map.get(wav_id).copied();
            // When `to_index()` returns `None`, skip the warning silently:
            // a non-decomposable WAV key (such as the "ZZ" LNOBJ marker) is
            // outside the WAV table's definition range and does not produce
            // a missing-definition warning.
            if audio.is_none()
                && let Some(key) = wav_id.0.to_index_in(base)
            {
                warnings.push(ProcessWarning::new(
                    None,
                    ProcessWarningKind::MissingWavDefinition {
                        key,
                        player,
                        lane,
                        tick,
                    },
                ));
            }
            audio
        };

        // Build the LN range table:
        // (player, lane) to Vec<(start_tick, end_tick)>.
        let ln_ranges = build_ln_ranges(paired_lns);

        // Slot requests in jbms-parser's channel processing order. End
        // markers occupy the slot without emitting an event.
        let mut placements: Vec<Option<Placement>> = Vec::new();

        // Visible notes (and LNOBJ markers). Notes covered by a long note move
        // to the BGM lane so their keysound keeps playing (beatoraja moves
        // them with `addBackGroundNote`); one exactly on the long-note start
        // only moves when its WAV differs from the start WAV, a same-WAV note
        // is redundant and dropped silently. Notes on an unterminated
        // long-note start are dropped: the start itself is removed at the end
        // of the song.
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            let tick = self.table.position_to_tick(ne.position);
            if consumed.contains(&i) {
                // Paired long-note starts are re-emitted from `paired_lns`;
                // only the end marker (its WAV equals `#LNOBJ`) occupies the
                // slot here, warning on conflict like a visible note does.
                let is_end = self
                    .bms
                    .gameplay
                    .ln_obj
                    .as_ref()
                    .is_some_and(|ln_obj| *ne.wav_id == **ln_obj);
                if is_end {
                    placements.push(Some(Placement {
                        tick,
                        player: ne.player,
                        lane: ne.lane,
                        kind: PlacementKind::LnEnd { warn: true },
                    }));
                }
                continue;
            }
            if matches!(ne.key_type, KeyType::Visible) {
                if unpaired_starts.contains(&(ne.player, ne.lane, tick)) {
                    continue;
                }
                let covered = paired_lns.iter().find(|ln| {
                    ln.player == ne.player
                        && ln.lane == ne.lane
                        && tick >= ln.tick
                        && tick <= ln.end_tick
                });
                if let Some(ln) = covered {
                    let audio = check_wav(&mut self.warnings, &ne.wav_id, ne.player, ne.lane, tick);
                    let redundant_start =
                        tick == ln.tick && audio == wav_map.get(&ln.wav_id).copied();
                    if !redundant_start {
                        if let Some(bgm_audio) = audio {
                            self.events.push(Event::bgm(tick, bgm_audio));
                        }
                        self.warnings.push(ProcessWarning::new(
                            None,
                            ProcessWarningKind::NoteCoveredByLongNote {
                                player: ne.player,
                                lane: ne.lane,
                                tick,
                            },
                        ));
                    }
                    continue;
                }
            }
            let audio = check_wav(&mut self.warnings, &ne.wav_id, ne.player, ne.lane, tick);
            let kind = match ne.key_type {
                KeyType::Visible => NoteKind::Normal,
                KeyType::Invisible => NoteKind::Invisible,
            };
            placements.push(Some(Placement {
                tick,
                player: ne.player,
                lane: ne.lane,
                kind: PlacementKind::Note { kind, audio },
            }));
        }

        // In LNOBJ mode, ch51-69 long-note channels are mutually exclusive with
        // LNOBJ (memo/10 leaves this undefined). Do not discard silently; keep
        // the data as ordinary visible notes.
        if self.bms.gameplay.ln_obj.is_some() {
            for lne in &self.bms.messages.long_note_events {
                let tick = self.table.position_to_tick(lne.position);
                let audio = check_wav(&mut self.warnings, &lne.wav_id, lne.player, lne.lane, tick);
                placements.push(Some(Placement {
                    tick,
                    player: lne.player,
                    lane: lne.lane,
                    kind: PlacementKind::Note {
                        kind: NoteKind::Normal,
                        audio,
                    },
                }));
            }
        }

        // Paired long notes: the start emits the long note; the end marker
        // overwrites the slot silently (jbms-parser does not warn here).
        for ln in paired_lns {
            let audio = check_wav(&mut self.warnings, &ln.wav_id, ln.player, ln.lane, ln.tick);
            placements.push(Some(Placement {
                tick: ln.tick,
                player: ln.player,
                lane: ln.lane,
                kind: PlacementKind::Long {
                    duration: ln.duration,
                    audio,
                },
            }));
            placements.push(Some(Placement {
                tick: ln.end_tick,
                player: ln.player,
                lane: ln.lane,
                kind: PlacementKind::LnEnd { warn: false },
            }));
        }

        // Mines.
        for me in &self.bms.messages.mine_events {
            placements.push(Some(Placement {
                tick: self.table.position_to_tick(me.position),
                player: me.player,
                lane: me.lane,
                kind: PlacementKind::Mine {
                    damage: Damage::new(mine_damage(me.raw_value)),
                },
            }));
        }

        // Resolve slot conflicts. A later note or long-note start wins
        // (beatoraja keeps only the last definition); mines never win.
        let mut occupied: HashMap<(u8, u8, u64), usize> = HashMap::new();
        for idx in 0..placements.len() {
            let Some(placement) = placements.get(idx).cloned().flatten() else {
                continue;
            };
            let key = (placement.player, placement.lane, placement.tick);
            match &placement.kind {
                // Invisible notes live on their own hidden slot in
                // jbms-parser; they never conflict.
                PlacementKind::Note {
                    kind: NoteKind::Invisible,
                    ..
                } => {}
                PlacementKind::Note { .. } | PlacementKind::Long { .. } => {
                    if let Some(&prev) = occupied.get(&key) {
                        self.warnings.push(ProcessWarning::new(
                            None,
                            ProcessWarningKind::DuplicateNotePosition {
                                player: placement.player,
                                lane: placement.lane,
                                tick: placement.tick,
                            },
                        ));
                        if let Some(prev_slot) = placements.get_mut(prev) {
                            *prev_slot = None;
                        }
                    }
                    occupied.insert(key, idx);
                }
                PlacementKind::LnEnd { warn } => {
                    if let Some(&prev) = occupied.get(&key) {
                        if *warn {
                            self.warnings.push(ProcessWarning::new(
                                None,
                                ProcessWarningKind::DuplicateNotePosition {
                                    player: placement.player,
                                    lane: placement.lane,
                                    tick: placement.tick,
                                },
                            ));
                        }
                        if let Some(prev_slot) = placements.get_mut(prev) {
                            *prev_slot = None;
                        }
                    }
                    occupied.insert(key, idx);
                }
                PlacementKind::Mine { .. } => {
                    let in_range = ln_ranges
                        .get(&(placement.player, placement.lane))
                        .is_some_and(|ranges| {
                            ranges.iter().any(|&(start, end)| {
                                placement.tick >= start && placement.tick <= end
                            })
                        });
                    // An unterminated long-note start occupies the slot in
                    // jbms-parser until it is removed at the end of the song,
                    // so a mine on it is dropped either way.
                    let on_unpaired_start = unpaired_starts.contains(&(
                        placement.player,
                        placement.lane,
                        placement.tick,
                    ));
                    if occupied.contains_key(&key) || in_range || on_unpaired_start {
                        self.warnings.push(ProcessWarning::new(
                            None,
                            ProcessWarningKind::MineNoteConflict {
                                player: placement.player,
                                lane: placement.lane,
                                tick: placement.tick,
                            },
                        ));
                        if let Some(slot) = placements.get_mut(idx) {
                            *slot = None;
                        }
                    } else {
                        occupied.insert(key, idx);
                    }
                }
            }
        }

        // Emit surviving placements.
        for placement in placements.into_iter().flatten() {
            let Some((note_side, note_lane)) =
                ChannelPos::new(placement.player, placement.lane).and_then(L::map_channel)
            else {
                continue;
            };
            match placement.kind {
                PlacementKind::Note { kind, audio } => self.events.push(Event::new(
                    placement.tick,
                    EventKind::Note(Note {
                        side: note_side,
                        lane: note_lane,
                        kind,
                        role: NoteRole::Normal,
                        audio: NoteAudio::from(audio),
                        ext: (),
                    }),
                )),
                PlacementKind::Long { duration, audio } => self.events.push(Event::new(
                    placement.tick,
                    EventKind::Note(Note {
                        side: note_side,
                        lane: note_lane,
                        kind: NoteKind::Long { duration },
                        role: NoteRole::Normal,
                        audio: NoteAudio::from(audio),
                        ext: (),
                    }),
                )),
                PlacementKind::Mine { damage } => self.events.push(Event::new(
                    placement.tick,
                    EventKind::Note(Note {
                        side: note_side,
                        lane: note_lane,
                        kind: NoteKind::Mine { damage },
                        role: NoteRole::Normal,
                        audio: NoteAudio::none(),
                        ext: (),
                    }),
                )),
                PlacementKind::LnEnd { .. } => {}
            }
        }
    }

    /// Collect BGM events into the event vector.
    pub fn collect_bgm(&mut self, wav_map: &BTreeMap<WavIndex, AudioIndex>) {
        for be in &self.bms.messages.bgm_events {
            if let Some(&audio) = wav_map.get(&be.wav_id) {
                self.events
                    .push(Event::bgm(self.table.position_to_tick(be.position), audio));
            }
        }
    }
}

/// Build the LN range lookup table.
fn build_ln_ranges(paired_lns: &[PairedLn]) -> HashMap<(u8, u8), Vec<(u64, u64)>> {
    let mut ln_ranges: HashMap<(u8, u8), Vec<(u64, u64)>> = HashMap::new();
    for ln in paired_lns {
        ln_ranges
            .entry((ln.player, ln.lane))
            .or_default()
            .push((ln.tick, ln.end_tick));
    }
    ln_ranges
}

/// One playable-note slot request built during note collection, resolved
/// against jbms-parser's channel order (visible notes, then long notes, then
/// mines) before any event is emitted.
#[derive(Clone)]
struct Placement {
    /// Absolute tick of the slot.
    tick: u64,
    /// Player number (1 or 2).
    player: u8,
    /// Original lane (1-9).
    lane: u8,
    /// What occupies the slot.
    kind: PlacementKind,
}

/// The occupant of a [`Placement`] slot.
#[derive(Clone)]
enum PlacementKind {
    /// Normal or invisible note.
    Note {
        /// Note kind (normal or invisible).
        kind: NoteKind,
        /// Resolved keysound, if the WAV reference resolves.
        audio: Option<AudioIndex>,
    },
    /// Long-note start; the duration reaches the paired end.
    Long {
        /// Hold duration in ticks.
        duration: u64,
        /// Resolved keysound of the start, if the WAV reference resolves.
        audio: Option<AudioIndex>,
    },
    /// Long-note end marker: occupies the slot, emits nothing. `warn` is set
    /// for LNOBJ markers, which warn on conflict like a visible note does
    /// (jbms-parser); ch51-69 ends overwrite silently.
    LnEnd {
        /// Whether a displaced note produces a warning.
        warn: bool,
    },
    /// Mine note.
    Mine {
        /// Damage value.
        damage: Damage,
    },
}

/// Convert a mine's raw Base36 value into a damage value.
///
/// - `Some(1295)` (`"ZZ"`) means instant death (`f64::INFINITY`).
/// - `Some(n)` gives `n / 2.0`.
/// - `None` (decoding failed) gives the default `1.0`.
fn mine_damage(raw: Option<u16>) -> f64 {
    match raw {
        None => 1.0,
        Some(1295) => f64::INFINITY,
        Some(n) => f64::from(n) / 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_kill_value_produces_infinite_damage() {
        assert!(mine_damage(Some(1295)).is_infinite());
    }

    #[test]
    fn normal_value_produces_half_damage() {
        assert!((mine_damage(Some(1)) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn undecodable_value_produces_default_damage() {
        assert!((mine_damage(None) - 1.0).abs() < f64::EPSILON);
    }
}
