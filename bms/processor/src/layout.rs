//! BMS mode-family layouts: channel-to-lane mapping through [`BmsLayout`].
//!
//! Each family is a zero-sized newtype that maps a decoded [`ChannelPos`]
//! `(player, lane)` pair to a `(NoteSide, Lane)` position.
//!
//! A family does **not** carry a key count: whether a chart is 5K or 7K depends
//! on the lanes its notes actually use, not on the family itself.

use std::num::NonZeroU8;

use bmsrs_chart::mode::{Lane, NoteSide};

/// Decoded BMS channel identifier: a `(player, lane)` pair extracted from the
/// raw BMS channel bytes (for example, `"19"` gives `{ player: 1, lane: 9 }`
/// and `"1A"` gives `{ player: 1, lane: 10 }`).
///
/// Construction validates `player` is in `{1, 2}` and `lane` is in `1..=35`, so
/// downstream code can safely call [`note_side`](Self::note_side) and
/// [`lane`](Self::lane) without additional checks. The lane value is the decoded
/// number (1-35), **not** the raw channel byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelPos {
    /// Player number (1 or 2).
    player: u8,
    /// Lane number (1-35; 1-9 are standard channels, 10-35 are extended
    /// channels).
    lane: u8,
}

impl ChannelPos {
    /// Create a new `ChannelPos` from a decoded `(player, lane)` pair.
    ///
    /// Returns `None` when `player` is not 1 or 2, or `lane` is outside 1-35.
    #[must_use]
    pub const fn new(player: u8, lane: u8) -> Option<Self> {
        if (player == 1 || player == 2) && lane >= 1 && lane <= 35 {
            Some(Self { player, lane })
        } else {
            None
        }
    }

    /// Player side (1 or 2).
    #[must_use]
    pub const fn player(self) -> u8 {
        self.player
    }

    /// Lane number (1-35).
    #[must_use]
    pub const fn lane(self) -> u8 {
        self.lane
    }

    /// Return the player side as a [`NoteSide`].
    #[must_use]
    pub const fn note_side(self) -> NoteSide {
        match self.player {
            1 => NoteSide::P1,
            _ => NoteSide::P2,
        }
    }
}

/// BMS side mapping: decode a [`ChannelPos`] into a `(NoteSide, Lane)` position
/// pair, or return `None` when the channel should be dropped.
///
/// The note kind (normal, long note, mine, or invisible) is set by the processor
/// after this mapping.
pub trait BmsLayout {
    /// Map a decoded BMS channel to a note position.
    #[must_use]
    fn map_channel(ch: ChannelPos) -> Option<(NoteSide, Lane)>;
}

/// Construct a `NonZeroU8` from a value known at the call site to be at least 1.
const fn nz(n: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(n)
}

/// BME family: covers `beat-5k`, `beat-7k`, `beat-10k`, and `beat-14k` (plus
/// the `dj-*` aliases). One mapping table serves all of them; a specific chart's
/// key count is determined by what its notes actually use.
///
/// Physical layout (left to right): `KEY1-5 | SC | KEY6-7` per side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bme;

impl BmsLayout for Bme {
    fn map_channel(ch: ChannelPos) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Scratch(nz(1)?),
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            // Extended channels (pomu2 family `1A`-`1Z` and so on, lanes 10-35).
            n if (10..=35).contains(&n) => Lane::Key(nz(n)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// Nanasi family: BME keys plus a foot pedal on channels `17`/`27`. Covers the
/// nanasi and Angolmois pedal variants (their channel mappings are identical;
/// differences in scratch rendering belong to the renderer, not the mapping).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Nanasi;

impl BmsLayout for Nanasi {
    fn map_channel(ch: ChannelPos) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Scratch(nz(1)?),
            7 => Lane::FootPedal,
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            // Extended channels (pomu2 family, lanes 10-35).
            n if (10..=35).contains(&n) => Lane::Key(nz(n)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// Native PMS family: a single-player 9-key layout whose KEY6-9 reuse 2P
/// channels `22-25`. Covers `popn-9k`, `popn-5k` (a subset), and the 3K subset
/// of `pomu-battle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pms;

impl BmsLayout for Pms {
    fn map_channel(ch: ChannelPos) -> Option<(NoteSide, Lane)> {
        let key = match (ch.note_side().as_u8(), ch.lane()) {
            (1, 1) => Lane::Key(nz(1)?),
            (1, 2) => Lane::Key(nz(2)?),
            (1, 3) => Lane::Key(nz(3)?),
            (1, 4) => Lane::Key(nz(4)?),
            (1, 5) => Lane::Key(nz(5)?),
            (2, 2) => Lane::Key(nz(6)?),
            (2, 3) => Lane::Key(nz(7)?),
            (2, 4) => Lane::Key(nz(8)?),
            (2, 5) => Lane::Key(nz(9)?),
            // Extended channels (pomu2 family, lanes 10-35).
            (_, n) if (10..=35).contains(&n) => Lane::Key(nz(n)?),
            _ => return None,
        };
        Some((NoteSide::P1, key))
    }
}

/// PMS BME-type family: 9 keys per side using the BME channel shape
/// (`KEY6=18 KEY7=19 KEY8=16 KEY9=17`). Note that channels `16`/`17`, which the
/// BME family reads as Scratch or idle, carry `KEY8`/`KEY9` here; that is why a
/// separate family is needed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PmsBme;

impl BmsLayout for PmsBme {
    fn map_channel(ch: ChannelPos) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Key(nz(8)?),
            7 => Lane::Key(nz(9)?),
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            // Extended channels (pomu2 family, lanes 10-35).
            n if (10..=35).contains(&n) => Lane::Key(nz(n)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// PMS layout variant: Standard (native PMS) or BME-type (PMS BME form).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmsLayout {
    /// Native PMS layout: KEY6-9 on 2P channels `22-25`.
    Standard,
    /// PMS BME-type layout: uses the BME channel shape
    /// (`KEY6=18 KEY7=19 KEY8=16 KEY9=17`).
    BmeType,
}

impl PmsLayout {
    /// Detect the PMS variant automatically from parsed BMS data.
    ///
    /// Scans `(player, lane)` pairs in note, long-note, and mine events:
    /// - `player == 2 && lane in 2..=5` indicates Standard (native PMS).
    /// - `player == 1 && lane in 6..=9` indicates BME-type.
    /// - When both indicators appear, Standard wins (a safe default).
    /// - When no event indicates either, returns Standard.
    #[must_use]
    pub fn detect(bms: &bmsrs_bms_parser::Bms) -> Self {
        let mut has_standard = false;
        let mut has_bme = false;

        for ev in &bms.messages.note_events {
            check_pms_channel(ev.player, ev.lane, &mut has_standard, &mut has_bme);
        }
        for ev in &bms.messages.long_note_events {
            check_pms_channel(ev.player, ev.lane, &mut has_standard, &mut has_bme);
        }
        for ev in &bms.messages.mine_events {
            check_pms_channel(ev.player, ev.lane, &mut has_standard, &mut has_bme);
        }

        if has_standard {
            Self::Standard
        } else if has_bme {
            Self::BmeType
        } else {
            Self::Standard
        }
    }
}

/// Check whether a single `(player, lane)` pair indicates the PMS Standard or
/// BME-type variant.
fn check_pms_channel(player: u8, lane: u8, std: &mut bool, bme: &mut bool) {
    if player == 2 && (2..=5).contains(&lane) {
        *std = true;
    }
    if player == 1 && (6..=9).contains(&lane) {
        *bme = true;
    }
}

/// DSC/FPP + OCT/FP family: a two-sided layout with up to two scratches and one
/// foot pedal. DSC/FPP (two scratches, no pedal) and OCT/FP (13 keys plus a
/// second scratch and a pedal) are both subsets of this family's key set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DscOctFp;

impl BmsLayout for DscOctFp {
    fn map_channel(ch: ChannelPos) -> Option<(NoteSide, Lane)> {
        Some(match (ch.note_side().as_u8(), ch.lane()) {
            (1, 1) => (NoteSide::P1, Lane::Key(nz(1)?)),
            (1, 2) => (NoteSide::P1, Lane::Key(nz(2)?)),
            (1, 3) => (NoteSide::P1, Lane::Key(nz(3)?)),
            (1, 4) => (NoteSide::P1, Lane::Key(nz(4)?)),
            (1, 5) => (NoteSide::P1, Lane::Key(nz(5)?)),
            (1, 6) => (NoteSide::P1, Lane::Scratch(nz(1)?)),
            (1, 8) => (NoteSide::P1, Lane::Key(nz(6)?)),
            (1, 9) => (NoteSide::P1, Lane::Key(nz(7)?)),
            (2, 1) => (NoteSide::P2, Lane::FootPedal),
            (2, 2) => (NoteSide::P2, Lane::Key(nz(1)?)),
            (2, 3) => (NoteSide::P2, Lane::Key(nz(2)?)),
            (2, 4) => (NoteSide::P2, Lane::Key(nz(3)?)),
            (2, 5) => (NoteSide::P2, Lane::Key(nz(4)?)),
            (2, 6) => (NoteSide::P2, Lane::Scratch(nz(2)?)),
            (2, 8) => (NoteSide::P2, Lane::Key(nz(5)?)),
            (2, 9) => (NoteSide::P2, Lane::Key(nz(6)?)),
            // Extended channels (pomu2 family, lanes 10-35).
            (s, n) if (10..=35).contains(&n) => {
                let side = match s {
                    1 => NoteSide::P1,
                    _ => NoteSide::P2,
                };
                (side, Lane::Key(nz(n)?))
            }
            _ => return None,
        })
    }
}
