//! `KaleidBeat` sidecar extraction.
//!
//! The processor keeps `KaleidBeat` declarations and channel objects out of the
//! format-neutral `Chart`. Callers that understand the dialect read them from
//! [`crate::Processed::kaleid`](crate::Processed::kaleid).

use bmsrs_bms_model::{Bms, Messages, Position};
use bmsrs_bms_values::{
    BmsBase, BmsChannel, BmsIndex, BodyCells, ExtensionChannel, OptionWords, WavIndex,
};

/// `KaleidBeat` sidecar attached to [`crate::Processed`].
///
/// `KaleidBeat` extension: this structure is not part of the BMS
/// specification or beatoraja. Ordinary charts leave
/// [`crate::Processed::kaleid`] as `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsKaleid {
    /// Chart-local declaration headers.
    pub declarations: bmsrs_bms_model::Kaleid,
    /// `#OPTION` lines carried by the dialect chart.
    ///
    /// The four named `KaleidBeat` words are a dialect extension; the header
    /// itself is a nanasi extension.
    pub options: Vec<OptionWords>,
    /// Extension-channel objects in source order.
    pub objects: Vec<KaleidObject>,
}

/// One object on a `KaleidBeat` extension channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KaleidObject {
    /// Measure number.
    pub measure: u16,
    /// Position within the measure.
    pub position: Position,
    /// Object payload.
    pub kind: KaleidObjectKind,
}

/// Payload of one `KaleidBeat` extension-channel object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KaleidObjectKind {
    /// Potion note; the value is the recovery percentage in base 36.
    Potion {
        /// Player side (1 or 2).
        side: u8,
        /// Lane number (1-9).
        lane: u8,
        /// Recovery percentage (not clamped).
        recovery: u32,
    },
    /// Display-only dummy tap.
    DummyTap {
        /// Player side (1 or 2).
        side: u8,
        /// Lane number (1-9).
        lane: u8,
        /// Referenced WAV index.
        wav: WavIndex,
    },
    /// Display-only dummy long note.
    DummyLong {
        /// Player side (1 or 2).
        side: u8,
        /// Lane number (1-9).
        lane: u8,
        /// Referenced WAV index.
        wav: WavIndex,
    },
    /// Display-only dummy mine.
    DummyMine {
        /// Player side (1 or 2).
        side: u8,
        /// Lane number (1-9).
        lane: u8,
        /// Referenced WAV index.
        wav: WavIndex,
    },
    /// Display-only dummy potion.
    DummyPotion {
        /// Player side (1 or 2).
        side: u8,
        /// Lane number (1-9).
        lane: u8,
        /// Referenced WAV index.
        wav: WavIndex,
    },
    /// Additional bar line.
    BarLine,
    /// Scratch free zone.
    FreeZone {
        /// Player side (1 or 2).
        side: u8,
        /// Whether the channel uses the paired start/end form.
        paired: bool,
        /// Referenced WAV index.
        wav: WavIndex,
    },
}

/// Build the `KaleidBeat` sidecar, or `None` when the chart has no dialect
/// declarations and no extension-channel objects.
#[must_use]
pub fn build(bms: &Bms) -> Option<BmsKaleid> {
    let objects = collect_objects(bms);
    let options = bms.gameplay.option.clone();
    if bms.kaleid.is_empty() && objects.is_empty() && options.is_empty() {
        return None;
    }
    Some(BmsKaleid {
        declarations: bms.kaleid.clone(),
        options,
        objects,
    })
}

/// Collect every object on an extension channel.
fn collect_objects(bms: &Bms) -> Vec<KaleidObject> {
    let base = bms.gameplay.base.unwrap_or(BmsBase::Base36);
    let mut objects = Vec::default();

    for (&measure, channels) in bms.messages.raw() {
        for (channel, bodies) in channels {
            let BmsChannel::Extension(extension) = *channel else {
                continue;
            };
            let body = Messages::merge_bodies(bodies);
            let cells = BodyCells::including_zero(&body);
            let total = u32::try_from(cells.total()).unwrap_or(u32::MAX);
            for cell in cells {
                if cell.text == "00" {
                    continue;
                }
                let Ok(index) = cell.index.try_into() else {
                    continue;
                };
                let position = Position::new(measure, index, total);
                let Some(kind) = object_kind(extension, cell.text, base) else {
                    continue;
                };
                objects.push(KaleidObject {
                    measure,
                    position,
                    kind,
                });
            }
        }
    }
    objects
}

/// Decode one cell into an object payload.
fn object_kind(extension: ExtensionChannel, text: &str, base: BmsBase) -> Option<KaleidObjectKind> {
    let wav = || parse_wav(text, base);
    match extension {
        ExtensionChannel::Potion { side, lane } => Some(KaleidObjectKind::Potion {
            side,
            lane,
            recovery: u32::from_str_radix(text, 36).ok()?,
        }),
        ExtensionChannel::DummyTap { side, lane } => Some(KaleidObjectKind::DummyTap {
            side,
            lane,
            wav: wav()?,
        }),
        ExtensionChannel::DummyLong { side, lane } => Some(KaleidObjectKind::DummyLong {
            side,
            lane,
            wav: wav()?,
        }),
        ExtensionChannel::DummyMine { side, lane } => Some(KaleidObjectKind::DummyMine {
            side,
            lane,
            wav: wav()?,
        }),
        ExtensionChannel::DummyPotion { side, lane } => Some(KaleidObjectKind::DummyPotion {
            side,
            lane,
            wav: wav()?,
        }),
        ExtensionChannel::BarLine => Some(KaleidObjectKind::BarLine),
        ExtensionChannel::FreeZone { side, paired } => Some(KaleidObjectKind::FreeZone {
            side,
            paired,
            wav: wav()?,
        }),
    }
}

/// Parse and normalize a two-character WAV index.
fn parse_wav(text: &str, base: BmsBase) -> Option<WavIndex> {
    let index: BmsIndex = text.parse().ok()?;
    Some(WavIndex(index.normalize(base)))
}
