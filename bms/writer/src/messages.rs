//! Channel line (message) serialization.

use bmsrs_bms_parser::BmsChannel;
use bmsrs_bms_parser::Messages;

/// Serialize all channel lines.
///
/// Output is ascending by (measure, channel). Merge semantics match
/// [`bmsrs_bms_parser`] parsing:
/// - channel `01` (BGM): multiple lines are kept independently (polyphony
///   support)
/// - channel `02` (measure length): only the last line is emitted
/// - all other channels: merged by position; non-`00` overrides earlier values,
///   and `00` preserves the position
pub fn write_messages(messages: &Messages) -> Vec<String> {
    let mut out = Vec::new();
    for (&measure, channels) in &messages.raw {
        for (channel, bodies) in channels {
            match channel {
                BmsChannel::Bgm => {
                    for body in bodies {
                        out.push(message_line(measure, *channel, body));
                    }
                }
                BmsChannel::MeasureLength => {
                    if let Some(body) = bodies.last() {
                        out.push(message_line(measure, *channel, body));
                    }
                }
                _ => {
                    let body = Messages::merge_bodies(bodies);
                    if !body.is_empty() {
                        out.push(message_line(measure, *channel, &body));
                    }
                }
            }
        }
    }
    out
}

/// Format one channel line (`#MMMCC:body`).
fn message_line(measure: u16, channel: BmsChannel, body: &str) -> String {
    let channel_str = channel.as_str();
    format!("#{measure:03}{channel_str}:{body}")
}
