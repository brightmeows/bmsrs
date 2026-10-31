//! BMS engine-specific custom event types.
//!
//! These events are converted by [`BmsProcessor`](crate::BmsProcessor) from BMS
//! channels (such as BGA opacity, ARGB color, TEXT, and OPTION) and stored in
//! the [`EventKind::Custom`](bmsrs_chart::EventKind::Custom) variant. The tick
//! is stored uniformly by [`Event`](bmsrs_chart::Event); variants of this type
//! do not contain a tick.

use bmsrs_chart::{BgaLayer, BgaResourceId, CustomEvent};

/// Chart events specific to the BMS format.
///
/// # Design principles
///
/// - No variant **contains** a `tick` field; the tick is stored uniformly by
///   [`Event`](bmsrs_chart::Event).
/// - Only engine-specific events live here; general concepts (notes, BPM, BGA,
///   and so on) are carried by the native variants of
///   [`Event`](bmsrs_chart::Event).
///
/// # Marker trait
///
/// This type implements [`CustomEvent`] (a method-less marker trait) and is used
/// as the `C` type parameter of
/// [`EventKind::Custom`](bmsrs_chart::EventKind::Custom).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BmsCustomEvent {
    /// BGA layer opacity change (channels `0B`-`0E`).
    ///
    /// Value range `01`-`FF` (hexadecimal); `00` means fully transparent and
    /// `FF` means fully opaque.
    BgaOpacity {
        /// Target BGA layer.
        layer: BgaLayer,
        /// Opacity (0-255).
        opacity: u8,
    },
    /// BGA layer ARGB color change (channels `A1`-`A4`).
    ///
    /// Each color component ranges over `00`-`FF`.
    BgaArgb {
        /// Target BGA layer.
        layer: BgaLayer,
        /// Alpha channel.
        a: u8,
        /// Red component.
        r: u8,
        /// Green component.
        g: u8,
        /// Blue component.
        b: u8,
    },
    /// BGA key binding (channel `A5` / `#SWBGA`).
    ///
    /// Binds a BGA resource to a key action and shows the corresponding frame
    /// when pressed.
    BgaKeyBound {
        /// BGA resource index (corresponds to a resource in the BMP definition).
        resource_id: BgaResourceId,
    },
    /// Timed text display (channel `99` / `#TEXTxx` / `#SONGxx`).
    TextDisplay {
        /// TEXT definition index.
        text_index: u32,
    },
    /// Per-position judge override (channel `A0` / `#EXRANK`).
    ///
    /// **Semantics not finalized**: the channel value's Base36 number is passed
    /// through directly, without looking up the `#EXRANKxx` definition table.
    /// The multiplication relationship with `#DEFEXRANK` will be settled after
    /// the overall judging system design; consumers should not interpret this
    /// field before then.
    JudgeOverride {
        /// Base36 value passed through directly (semantics not finalized; see
        /// above).
        rank: u64,
    },
    /// Player option change (channel `A6` / `#CHANGEOPTION`).
    OptionChange {
        /// Option ID.
        option_id: u64,
        /// Option value.
        value: String,
    },
    /// BGM channel volume (channel `97`).
    BgmVolume {
        /// Volume (`01`-`FF`; `FF` means original volume).
        volume: u8,
    },
    /// KEY channel volume (channel `98`).
    KeyVolume {
        /// Volume (`01`-`FF`; `FF` means original volume).
        volume: u8,
    },
    /// Video seek (channel `05` / `#SEEK`).
    VideoSeek {
        /// Video target position in milliseconds, from the `#SEEK` definition
        /// table; undefined IDs are skipped.
        position: u64,
    },
}

impl CustomEvent for BmsCustomEvent {}
