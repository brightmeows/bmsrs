//! Example: parse a chart file and simulate playback with the player.
//!
//! Run:
//!
//! ```sh
//! cargo run --example quick_start -- path/to/chart.bms
//! ```
//!
//! This mirrors the README quick start: read the file, process it into the
//! unified `Chart`, and advance the player by one second.

#![expect(clippy::print_stdout, reason = "example prints to stdout by design")]

use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: quick_start <chart.bms>")?;

    let parsed = bmsrs::bms::parse_file(path)?;
    println!("parse warnings: {}", parsed.warnings.len());

    let processed = bmsrs::bms::process(&parsed.value)?;
    println!("events: {}", processed.value.chart.data().events().len());

    let mut player = bmsrs::player::Player::new(processed.value.chart);
    player.advance(Duration::from_secs(1));
    println!("current tick: {}", player.current_tick());
    println!("chart duration: {:?}", player.duration());
    Ok(())
}
