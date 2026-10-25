//! [`BmsChannel`] 与通道号字符串的互转。

use bms_tokenizer::BmsChannel;

/// 将已知非音符通道变体映射回通道号字符串。
///
/// 携带原始索引的变体（音符 / 未知通道）由调用方通过
/// [`BmsChannel`] 携带的索引自行格式化。
#[must_use]
pub const fn fixed_channel_str(ch: &BmsChannel) -> Option<&'static str> {
    match ch {
        BmsChannel::Bgm => Some("01"),
        BmsChannel::MeasureLength => Some("02"),
        BmsChannel::BpmChange => Some("03"),
        BmsChannel::BgaBase => Some("04"),
        BmsChannel::Seek => Some("05"),
        BmsChannel::BgaPoor => Some("06"),
        BmsChannel::BgaLayer => Some("07"),
        BmsChannel::ExtendedBpm => Some("08"),
        BmsChannel::Stop => Some("09"),
        BmsChannel::BgaLayer2 => Some("0A"),
        BmsChannel::BgaBaseOpacity => Some("0B"),
        BmsChannel::BgaLayerOpacity => Some("0C"),
        BmsChannel::BgaLayer2Opacity => Some("0D"),
        BmsChannel::BgaPoorOpacity => Some("0E"),
        BmsChannel::BgmVolume => Some("97"),
        BmsChannel::KeyVolume => Some("98"),
        BmsChannel::Text => Some("99"),
        BmsChannel::Judge => Some("A0"),
        BmsChannel::BgaArgbBase => Some("A1"),
        BmsChannel::BgaArgbLayer => Some("A2"),
        BmsChannel::BgaArgbLayer2 => Some("A3"),
        BmsChannel::BgaArgbPoor => Some("A4"),
        BmsChannel::BgaKeyBound => Some("A5"),
        BmsChannel::Option => Some("A6"),
        BmsChannel::Scroll => Some("SC"),
        BmsChannel::Speed => Some("SP"),
        BmsChannel::Note(_) | BmsChannel::Unknown(_) => None,
    }
}
