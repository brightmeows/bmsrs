#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;

use bmsrs_chart::layout::{
    Bme, BmsLayout, BmsonLayout, DscOctFp, GenericLayout, Nanasi, Pms, PmsBme,
};
use bmsrs_chart::mode::{BmsChannel, Lane, PlayerSide};
use bmsrs_chart::note::{NoteData, NoteKind};

/// Shorthand to construct a valid [`BmsChannel`] in tests.
fn ch(player: u8, lane: u8) -> BmsChannel {
    BmsChannel::new(player, lane).unwrap_or_else(|| panic!("invalid BMS channel"))
}

const fn key(n: u8) -> Option<Lane> {
    Some(Lane::Key(match NonZeroU8::new(n) {
        Some(v) => v,
        None => return None,
    }))
}

const SC1: Lane = Lane::Scratch(match NonZeroU8::new(1) {
    Some(v) => v,
    None => panic!("unreachable"),
});
const SC2: Lane = Lane::Scratch(match NonZeroU8::new(2) {
    Some(v) => v,
    None => panic!("unreachable"),
});
const PEDAL: Lane = Lane::FootPedal;

#[test]
fn bme_bms_maps_keys_scratch_and_key7() {
    assert_eq!(
        Bme::map_channel(ch(1, 1)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Bme::map_channel(ch(1, 5)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(5).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Bme::map_channel(ch(1, 6)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: SC1,
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Bme::map_channel(ch(1, 8)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(6).unwrap(),
            kind: NoteKind::Normal
        })
    );
    // KEY7 (channel 19) — regressed in the old Beat7k (lane 9 dropped)
    assert_eq!(
        Bme::map_channel(ch(1, 9)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(7).unwrap(),
            kind: NoteKind::Normal
        })
    );
    // FREE zone (channel 17) is unmapped in the BME family
    assert_eq!(Bme::map_channel(ch(1, 7)), None);
}

#[test]
fn bme_bms_maps_second_player_side() {
    assert_eq!(
        Bme::map_channel(ch(2, 1)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Bme::map_channel(ch(2, 6)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: SC1,
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Bme::map_channel(ch(2, 9)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(7).unwrap(),
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn bme_bmson_x_aligns_scratch_with_bms() {
    let layout = Bme;
    // x=8 is the 1P scratch and must equal BMS channel 16's position.
    assert_eq!(layout.map_x(8), Bme::map_channel(ch(1, 6)));
    assert_eq!(
        layout.map_x(8),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: SC1,
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        layout.map_x(6),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(6).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        layout.map_x(7),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(7).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        layout.map_x(9),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        layout.map_x(16),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: SC1,
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn bme_bmson_covers_5k_as_subset() {
    let layout = Bme;
    assert_eq!(
        layout.map_x(5),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(5).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        layout.map_x(8),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: SC1,
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn bms_channel_rejects_invalid_player() {
    assert_eq!(BmsChannel::new(3, 1), None);
    assert_eq!(BmsChannel::new(0, 1), None);
}

#[test]
fn nanasi_adds_foot_pedal_on_channel_17() {
    assert_eq!(
        Nanasi::map_channel(ch(1, 6)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: SC1,
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Nanasi::map_channel(ch(1, 9)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(7).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Nanasi::map_channel(ch(1, 7)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: PEDAL,
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Nanasi::map_channel(ch(2, 7)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: PEDAL,
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn pms_bms_spans_both_player_channels_into_single_side() {
    // KEY1-5 on 1P channels 11-15 → Player1
    assert_eq!(
        Pms::map_channel(ch(1, 1)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Pms::map_channel(ch(1, 5)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(5).unwrap(),
            kind: NoteKind::Normal
        })
    );
    // KEY6-9 on 2P channels 22-25 → still Player1 (single-player mode)
    assert_eq!(
        Pms::map_channel(ch(2, 2)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(6).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Pms::map_channel(ch(2, 5)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(9).unwrap(),
            kind: NoteKind::Normal
        })
    );
    // Channel 21 (2P lane 1) is unused by PMS.
    assert_eq!(Pms::map_channel(ch(2, 1)), None);
}

#[test]
fn pms_bmson_popn_9k_maps_nine_keys() {
    let layout = Pms;
    assert_eq!(
        layout.map_x(1),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        layout.map_x(9),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(9).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(layout.map_x(10), None);
}

#[test]
fn pms_bme_type_reinterprets_channel_16_17_as_keys() {
    assert_eq!(
        PmsBme::map_channel(ch(1, 8)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(6).unwrap(),
            kind: NoteKind::Normal
        })
    ); // KEY6 (ch 18)
    assert_eq!(
        PmsBme::map_channel(ch(1, 9)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(7).unwrap(),
            kind: NoteKind::Normal
        })
    ); // KEY7 (ch 19)
    assert_eq!(
        PmsBme::map_channel(ch(1, 6)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(8).unwrap(),
            kind: NoteKind::Normal
        })
    ); // KEY8 (ch 16)
    assert_eq!(
        PmsBme::map_channel(ch(1, 7)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(9).unwrap(),
            kind: NoteKind::Normal
        })
    ); // KEY9 (ch 17)
}

#[test]
fn pms_bme_type_maps_second_player_side() {
    assert_eq!(
        PmsBme::map_channel(ch(2, 1)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        PmsBme::map_channel(ch(2, 7)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(9).unwrap(),
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn dsc_oct_fp_maps_dual_scratch_and_pedal() {
    assert_eq!(
        DscOctFp::map_channel(ch(1, 6)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: SC1,
            kind: NoteKind::Normal
        })
    ); // SC1 (ch 16)
    assert_eq!(
        DscOctFp::map_channel(ch(1, 1)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        DscOctFp::map_channel(ch(2, 2)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    ); // KEY8 (ch 22)
    assert_eq!(
        DscOctFp::map_channel(ch(2, 6)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: SC2,
            kind: NoteKind::Normal
        })
    ); // SC2 (ch 26)
    assert_eq!(
        DscOctFp::map_channel(ch(2, 1)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: PEDAL,
            kind: NoteKind::Normal
        })
    ); // pedal (ch 21)
}

#[test]
fn generic_maps_keys_left_to_right() {
    let layout = GenericLayout { keys: 4 };
    assert_eq!(
        layout.map_x(1),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(1).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        layout.map_x(4),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(4).unwrap(),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(layout.map_x(5), None);
}
