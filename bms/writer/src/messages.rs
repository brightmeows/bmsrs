//! 通道行（消息）的序列化。

use std::collections::BTreeMap;

use bms_parser::BmsChannel;
use bms_parser::Messages;

/// 序列化全部通道行。
///
/// 按（小节，通道）升序输出。合并语义与 [`bms_parser`] 的解析一致：
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
                    if let Some(body) = merge_bodies(bodies) {
                        out.push(message_line(measure, *channel, &body));
                    }
                }
            }
        }
    }
    out
}

/// 将一个（小节，通道）的多行正文按位置合并为单行。
///
/// 槽位解析按各行自身长度（`len / 2`）进行；后续行的非 `00`
/// 值覆盖先前值，`00` 不清除已有值。全部为空时返回 `None`。
fn merge_bodies(bodies: &[String]) -> Option<String> {
    let mut slots: BTreeMap<usize, String> = BTreeMap::new();
    for body in bodies {
        for (slot, value) in chunk_2char(body).enumerate() {
            if value != "00" || !slots.contains_key(&slot) {
                slots.insert(slot, value.to_owned());
            }
        }
    }
    if slots.is_empty() {
        return None;
    }
    let len = slots.keys().next_back().copied()? + 1;
    Some(bms_parser::BmsMessage::encode_body(
        (0..len).map(|i| slots.get(&i).map_or("00", String::as_str)),
    ))
}

/// 将正文按 2 字符切块迭代；尾部奇数字符忽略。
fn chunk_2char(body: &str) -> impl Iterator<Item = &str> {
    body.as_bytes().chunks(2).map(|chunk| {
        // chunks 长度为 1 仅出现在奇数长度正文的尾部；BMS 正文的
        // 值恒为 2 字符，此处仅处理防御性场景。
        std::str::from_utf8(chunk).unwrap_or("00")
    })
}

/// 格式化一条通道行（`#MMMCC:body`）。
fn message_line(measure: u16, channel: BmsChannel, body: &str) -> String {
    let channel_str = channel.as_str();
    format!("#{measure:03}{channel_str}:{body}")
}
