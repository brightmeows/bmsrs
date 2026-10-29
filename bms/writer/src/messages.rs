//! 通道行（消息）的序列化。

use bmsrs_bms_parser::BmsChannel;
use bmsrs_bms_parser::Messages;

/// 序列化全部通道行。
///
/// 按（小节，通道）升序输出。合并语义与 [`bmsrs_bms_parser`] 的解析一致：
/// - 通道 `01`（BGM）：多行独立保留（多声部支持）
/// - 通道 `02`（小节长）：仅输出最后一行
/// - 其余通道：按位置合并，非 `00` 覆盖先前值，`00` 保留位置
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

/// 格式化一条通道行（`#MMMCC:body`）。
fn message_line(measure: u16, channel: BmsChannel, body: &str) -> String {
    let channel_str = channel.as_str();
    format!("#{measure:03}{channel_str}:{body}")
}
