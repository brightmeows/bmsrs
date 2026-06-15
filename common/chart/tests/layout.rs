#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::layout::{
    Beat5k, Beat7k, Beat10k, Beat14k, GenericLayout, Layout, Popn5k, Popn9k,
};

#[test]
fn beat7k_lane_count() {
    assert_eq!(Beat7k.lane_count(), 8);
}

#[test]
fn beat5k_lane_count() {
    assert_eq!(Beat5k.lane_count(), 6);
}

#[test]
fn beat14k_lane_count() {
    assert_eq!(Beat14k.lane_count(), 16);
}

#[test]
fn beat10k_lane_count() {
    assert_eq!(Beat10k.lane_count(), 12);
}

#[test]
fn popn9k_lane_count() {
    assert_eq!(Popn9k.lane_count(), 9);
}

#[test]
fn popn5k_lane_count() {
    assert_eq!(Popn5k.lane_count(), 5);
}

#[test]
fn generic_layout_lane_count() {
    let layout = GenericLayout { keys: 4 };
    assert_eq!(layout.lane_count(), 4);
}

#[test]
fn layout_clone_and_eq() {
    let a = Beat7k;
    let b = a;
    assert_eq!(a, b);
}
