//! 跨 bmson 版本（v0、v1、v2）共享的类型。

use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DefaultOnNull, serde_as};
use std::fmt;
use std::path::Path;

/// 指定输入布局的游戏模式提示。
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModeHint {
    /// beat-5k（5 个按键 + 1 个转盘）。
    Beat5k,
    /// beat-7k（7 个按键 + 1 个转盘）。
    #[default]
    Beat7k,
    /// beat-10k（每位玩家 5 个按键 + 转盘，双人）。
    Beat10k,
    /// beat-14k（每位玩家 7 个按键 + 转盘，双人）。
    Beat14k,
    /// popn-5k。
    Popn5k,
    /// popn-9k。
    Popn9k,
    /// dj-5k-only。
    Dj5kOnly,
    /// dj-ruby。
    DjRuby,
    /// dj-5k。
    Dj5k,
    /// dj-7k。
    Dj7k,
    /// dj-10k。
    Dj10k,
    /// dj-14k。
    Dj14k,
    /// dj-andromeda。
    DjAndromeda,
    /// 通用 n 按键布局，按键从左到右编号 1..n。
    /// 序列化为 `"generic-{n}k"`。
    Generic(u64),
    /// 规范列表以外的任何其他（扩展）模式提示。
    Other(String),
}

impl fmt::Display for ModeHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Beat5k => f.write_str("beat-5k"),
            Self::Beat7k => f.write_str("beat-7k"),
            Self::Beat10k => f.write_str("beat-10k"),
            Self::Beat14k => f.write_str("beat-14k"),
            Self::Popn5k => f.write_str("popn-5k"),
            Self::Popn9k => f.write_str("popn-9k"),
            Self::Dj5kOnly => f.write_str("dj-5k-only"),
            Self::DjRuby => f.write_str("dj-ruby"),
            Self::Dj5k => f.write_str("dj-5k"),
            Self::Dj7k => f.write_str("dj-7k"),
            Self::Dj10k => f.write_str("dj-10k"),
            Self::Dj14k => f.write_str("dj-14k"),
            Self::DjAndromeda => f.write_str("dj-andromeda"),
            Self::Generic(n) => write!(f, "generic-{n}keys"),
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl core::str::FromStr for ModeHint {
    type Err = core::convert::Infallible;

    #[expect(
        clippy::string_slice,
        reason = "\"generic-\" and \"keys\"/\"k\" are ASCII; byte indexing is safe"
    )]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "beat-5k" => Self::Beat5k,
            "beat-7k" => Self::Beat7k,
            "beat-10k" => Self::Beat10k,
            "beat-14k" => Self::Beat14k,
            "popn-5k" => Self::Popn5k,
            "popn-9k" => Self::Popn9k,
            "dj-5k-only" => Self::Dj5kOnly,
            "dj-ruby" => Self::DjRuby,
            "dj-5k" => Self::Dj5k,
            "dj-7k" => Self::Dj7k,
            "dj-10k" => Self::Dj10k,
            "dj-14k" => Self::Dj14k,
            "dj-andromeda" => Self::DjAndromeda,
            _ if s.starts_with("generic-") && s.ends_with("keys") => {
                let inner = &s[8..s.len() - 4]; // 去掉 "generic-" 前缀和 "keys" 后缀
                inner
                    .parse()
                    .map_or_else(|_| Self::Other(s.to_owned()), Self::Generic)
            }
            _ if s.starts_with("generic-") && s.ends_with('k') => {
                let inner = &s[8..s.len() - 1]; // 去掉 "generic-" 前缀和 "k" 后缀（遗留兼容）
                inner
                    .parse()
                    .map_or_else(|_| Self::Other(s.to_owned()), Self::Generic)
            }
            _ => Self::Other(s.to_owned()),
        })
    }
}

impl Serialize for ModeHint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ModeHint {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

/// 长音类型提示。
///
/// 可在谱面级设置（[`crate::ChartData::ln_type_hint`]），并可按音符覆盖
/// （[`NoteEvent::ln_type_hint`]）。
///
/// | 值 | 含义 | 来源 |
/// |---|---|---|
/// | `ln` | 仅在按下时判定 | v2 标准 |
/// | `cn` | 在按下和释放时都判定 | v2 标准 |
/// | `hcn` | 地狱充电音，按住期间判定更严格 | beatoraja 扩展 |
///
/// ⚠️ `hcn` 是 beatoraja 扩展，不属于 bmson v2 核心规范。
/// 播放器若不支持 HCN，应将其降级为 `cn` 处理。
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LnType {
    /// 长音（LN）—— 仅在按下时判定。
    #[default]
    Ln,
    /// 充电音（CN）—— 在按下和释放时都判定。
    Cn,
    /// 地狱充电音（HCN）—— beatoraja 扩展，非 v2 标准。
    Hcn,
}

/// 长音判定提示（`"normal"` 或 `"ticks"`）。
///
/// | 值 | 含义 |
/// |---|---|
/// | `normal` | 仅判定音符本身 |
/// | `ticks` | 按住期间额外判定若干脉冲 |
///
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LnJudge {
    /// 仅判定音符本身。
    #[default]
    Normal,
    /// 按住期间额外判定若干脉冲。
    Ticks,
}

/// 长音血量槽（life）提示（`"normal"` 或 `"ticks"`）。
///
/// | 值 | 含义 |
/// |---|---|
/// | `normal` | 仅音符本身恢复血量 |
/// | `ticks` | 按住期间额外脉冲恢复血量 |
///
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LnLife {
    /// 仅音符本身恢复血量。
    #[default]
    Normal,
    /// 按住期间额外脉冲恢复血量。
    Ticks,
}

/// beatoraja 长音模式（数值型，v0 扩展）。
///
/// | 值 | 变体 | 含义 |
/// |---|---|---|
/// | `1` | `Ln` | LN —— 仅按下 |
/// | `2` | `Cn` | CN —— 按下 + 释放 |
/// | `3` | `Hcn` | HCN —— 地狱充电音 |
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
#[non_exhaustive]
pub enum LnMode {
    /// LN（1）—— 仅按下。
    Ln,
    /// CN（2）—— 按下 + 释放。
    Cn,
    /// HCN（3）—— 地狱充电音。
    Hcn,
}

impl Serialize for LnMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Ln => serializer.serialize_u64(1),
            Self::Cn => serializer.serialize_u64(2),
            Self::Hcn => serializer.serialize_u64(3),
        }
    }
}

impl<'de> Deserialize<'de> for LnMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let n = u64::deserialize(deserializer)?;
        match n {
            1 => Ok(Self::Ln),
            2 => Ok(Self::Cn),
            3 => Ok(Self::Hcn),
            other => Err(de::Error::invalid_value(
                Unexpected::Unsigned(other),
                &"1 (LN), 2 (CN), or 3 (HCN)",
            )),
        }
    }
}

/// [`SoundChannel`] 中的单个音符（可玩或 BGM）。
///
/// 在 v2.0.0-rc1 中由 `Note` 重命名为 `NoteEvent`。
///
///
/// # 跨版本共享字段
///
/// | 字段 | 类型 | 说明 |
/// |---|---|---|
/// | `x` | `u64` | 玩家通道（`0` = BGM，`>0` = 可玩通道）|
/// | `y` | `u64` | 脉冲偏移 |
/// | `l` | `u64` | 脉冲长度（`0` = 短音，`>0` = 长音）|
/// | `c` | `bool` | 续接标志（音频重新播放行为）|
///
/// # v2 专有可选字段
///
/// 以下字段仅存在于 v2.0.0-rc1 及以上版本；从 v0/v1 文件反序列化时为 `None`：
///
/// | 字段 | 类型 | 说明 |
/// |---|---|---|
/// | `up` | `bool` | 释放音 / BSS 标志 |
/// | `ln_type_hint` | [`LnType`] | 按音符的长音类型覆盖 |
/// | `ln_judge_hint` | [`LnJudge`] | 按音符的长音判定覆盖 |
/// | `ln_life_hint` | [`LnLife`] | 按音符的长音血量覆盖 |
/// | `vol` | `i8` | 音量（百分比，DJ.NEXT）|
/// | `pan` | `i8` | 声像（DJ.NEXT）|
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteEvent {
    /// 玩家通道（`0` = BGM，`>0` = 可玩按键/列）。
    /// 字段缺失时默认为 `0`。也接受 `null`，视为 `0`（BGM）。
    ///
    /// 从通道编号到屏幕列的确切映射取决于
    /// [`crate::ChartData::mode_hint`]：
    ///
    /// | 模式 | 通道 |
    /// |---|---|
    /// | `beat-7k` | 1–7 = 按键，8 = 转盘 |
    /// | `beat-5k` | 1–5 = 按键，8 = 转盘 |
    /// | `popn-9k` | 1–9 = 按键 |
    /// | `generic-nkeys` | 1…n 从左到右 |
    #[serde(default)]
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub x: u64,

    /// 此音符的脉冲偏移。
    ///
    /// bmson 中所有计时都基于脉冲（tick），而非秒。
    /// 每四分音符的脉冲数由 [`crate::ChartData::resolution`] 给出（默认 240）。
    pub y: u64,

    /// 音符长度（脉冲数）。
    ///
    /// | 值 | 含义 |
    /// |---|---|
    /// | `0` | 短（普通）音符 —— 无保持 |
    /// | `>0` | 长音，从脉冲 `y` 持续到 `y + l` |
    pub l: u64,

    /// 续接标志 —— 是否重新播放音频切片。
    ///
    /// - `true` —— **续接**（不重新播放）：音频从上一个切片继续播放。
    /// - `false` —— **不续接**：在此音符的切片点重新播放音频。
    pub c: bool,

    /// 按音符的长音类型覆盖（beatoraja 扩展，数值型）。
    ///
    /// | 值 | 含义 |
    /// |---|---|---|
    /// | `1` | LN —— 仅按下 |
    /// | `2` | CN —— 按下 + 释放 |
    /// | `3` | HCN —— 地狱充电音 |
    ///
    /// v2 等价字段另见 [`NoteEvent::ln_type_hint`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<LnMode>,

    /// 释放音 / BSS（Back-Spin-Scratch，倒搓盘）标志。
    ///
    /// 对于 CN（充电音）或 BSS，在释放位置放置一个 `up: true` 且长度为零的
    /// `NoteEvent`。
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<bool>,

    /// 谱面级长音类型提示的按音符覆盖。
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_type_hint: Option<LnType>,

    /// 谱面级长音判定提示的按音符覆盖。
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_judge_hint: Option<LnJudge>,

    /// 谱面级长音血量提示的按音符覆盖。
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_life_hint: Option<LnLife>,

    /// 有符号百分比表示的音符音量（DJ.NEXT 扩展）。
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vol: Option<i8>,

    /// 有符号百分比表示的音符声像（DJ.NEXT 扩展）。
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pan: Option<i8>,
}

impl NoteEvent {
    /// 如果此音符是 BGM 音符（不可玩）则返回 `true`。
    #[must_use]
    #[inline]
    pub const fn is_bgm(&self) -> bool {
        self.x == 0
    }
}

/// **音频通道** —— 单个音频文件及其关联音符。
///
/// Bmson 基于通道：每个 [`SoundChannel`] 将一个音频文件（`name`）与
/// 所有引用它的 [`NoteEvent`] 绑定在一起。播放器在音符位置处对音频文件
/// 切片，并为每个音符播放相应的片段。
///
///
/// # 字段命名
///
/// 根模块使用 **v2** 字段名 `note_events`。
/// [`crate::v1`] 模块有自己的 [`SoundChannel`](crate::v1::SoundChannel)，
/// 使用 v1 字段名 `notes`（通过 `#[serde(rename = "notes")]` 别名）。
///
/// # 切片行为
///
/// 1. 从 `note_events` 收集所有唯一的脉冲偏移。
/// 2. 排序并转换为实际时间（使用 BPM 和分辨率）。
/// 3. 在这些时间点对音频文件切片。
/// 4. 每个音符分配到以其脉冲为起点的切片。
///    `c: false` 的音符会在该点触发重新播放。
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct SoundChannel<'a> {
    /// 音频文件名（相对路径，扩展名可省略）。
    ///
    /// 扩展名缺失时，播放器会搜索兼容的音频文件（`.wav`、`.ogg`、`.m4a`）。
    ///
    /// # 安全性
    ///
    /// 实现方**必须**防止目录穿越和绝对路径
    /// （例如 `../secret.txt`、`/etc/passwd`）。
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,

    /// 引用此音频文件的音符。
    ///
    /// 此字段序列化为 `note_events`（v2 约定）。
    pub note_events: Vec<NoteEvent>,
}

/// **小节线**事件，标记谱面中的小节边界。
///
/// Bmson 原生没有小节或拍号的概念。小节线是显式标记，播放器可在屏幕上渲染。
///
///
/// # 字段行为
///
/// | 值 | 含义 |
/// |---|---|
/// | 空数组（`[]`）| 不显示小节线 —— 滚动效果类似 [100% minimoo-G](https://www.youtube.com/watch?v=f1VBBNrSdgk) |
/// | `null` | 视为 4/4 常规拍（每 4 个四分音符一条小节线 = 默认分辨率下 960 脉冲）|
/// | `\[{ y }\]` | 在给定脉冲偏移处放置小节线 |
///
/// # 勘误
///
/// v0.2.1 有一个额外的 `k` 字段（现已移除）。往返经过 v0 时请使用
/// [`crate::v0::BarLine`]。
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct BarLine {
    /// 此小节线的脉冲偏移。
    ///
    /// 第一条小节线（`y: 0`）可省略；是否渲染由播放器决定。
    pub y: u64,
}

/// 改变乐曲速度的 **BPM 变更**事件。
///
/// 在脉冲 `y` 处，播放 BPM 更新为 `bpm`。
/// 如果多个 `BpmEvent` 共享同一脉冲，**最后一个**生效
/// （与 BMS 行为一致）。
///
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct BpmEvent {
    /// 速度变更生效的脉冲偏移。
    pub y: u64,
    /// 新速度，单位为每分钟拍数（BPM）。
    pub bpm: f64,
}

/// 暂停音乐滚动一段时长的 **停止**（STOP）事件。
///
/// 当多个 `StopEvent` 共享同一脉冲时，时长**累加**
/// （例如两个 240 和 960 脉冲的停止 = 总计 1200）。
///
///
/// # 同一脉冲上的事件顺序
///
/// 1. `Note` / `BGAEvent`
/// 2. `BpmEvent`
/// 3. `StopEvent`
///
/// 停止所在脉冲处的 BPM 值用于计算暂停的实际时长。
/// 与停止在同一脉冲上的音符必须在滚动停止**之前**按下。
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct StopEvent {
    /// 停止开始的脉冲偏移。
    pub y: u64,
    /// 停止时长（脉冲数，非秒）。
    ///
    /// 时长即"被跳过的音乐时间量"，通过当前 BPM 转换为实际时间。
    pub duration: u64,
}

/// BGA 图片或视频资源的头部条目。
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BGAHeader<'a> {
    /// 被 [`BGAEvent::id`] 引用的数字标识符。
    ///
    /// 同一文件内重复的 `id` 值视为警告；最后一次出现者生效。
    ///
    /// **注意：** v0.2.1 使用大写键 `ID` 表示此字段。
    /// `alias` 属性允许同时反序列化 `id` 和 `ID`。
    #[serde(alias = "ID")]
    pub id: u64,
    /// 图片或视频资源的文件路径。
    ///
    /// 支持的格式：`PNG`（图片）、`WebM`（视频，音轨被忽略）。
    /// 推荐分辨率：1280×720；1920×1080 也可接受。
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,
}

/// 引用 [`BGAHeader`] 中资源的 BGA 显示事件。
///
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct BGAEvent {
    /// 此图片/视频变为可见的脉冲偏移。
    pub y: u64,
    /// 要显示的 [`BGAHeader`] 中资源的标识符。
    pub id: u64,
}

/// 背景动画（BGA）数据。
///
/// 包含三条独立的事件轨道，播放器可进行合成：
///
/// | 轨道 | 用途 |
/// |---|---|
/// | `bga_events` | 主背景动画 |
/// | `layer_events` | 叠加在主 BGA 之上的图层 |
/// | `poor_events` | 玩家漏击音符时显示的 POOR 动画 |
///
///
/// # 透明度说明
///
/// 与 BMS 的 `#LAYER` 通道不同，`layer_events` 中的黑色像素**不会**自动变为透明。
/// 如需透明效果，请使用带真实 alpha 通道的 PNG。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BGA<'a> {
    /// 资源声明（图片/视频 id → 文件名映射）。
    #[serde(rename = "bga_header", alias = "bgaHeader")]
    pub bga_header: Vec<BGAHeader<'a>>,
    /// 主背景动画序列。
    #[serde(rename = "bga_events", alias = "bgaNotes")]
    pub bga_events: Vec<BGAEvent>,
    /// 叠加在主 BGA 之上的图层序列。
    #[serde(rename = "layer_events", alias = "layerNotes")]
    pub layer_events: Vec<BGAEvent>,
    /// POOR（漏击）动画序列。
    #[serde(rename = "poor_events", alias = "poorNotes")]
    pub poor_events: Vec<BGAEvent>,
}

/// 滚动速度倍率事件（beatoraja 扩展）。
///
/// 类似于 BMS 的 `#SCROLL` / `#SPEED`。
///
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct ScrollEvent {
    /// 脉冲偏移。
    pub y: u64,
    /// 速度倍率（负值 → 反向滚动）。
    pub rate: f64,
}

/// 地雷（landmine）通道（beatoraja 扩展）。
///
/// 每个通道将共享同一音频文件的地雷音符归为一组。
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct MineChannel<'a> {
    /// 音频文件名（地雷被触发时播放）。
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,
    /// 此通道中的地雷音符。
    pub notes: Vec<MineNote>,
}

/// 单个地雷音符（beatoraja 扩展）。
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct MineNote {
    /// 玩家通道（语义同 [`NoteEvent::x`]）。
    pub x: u64,
    /// 脉冲偏移。
    pub y: u64,
    /// 生命值伤害（支持小数值）。
    pub damage: f64,
}

/// 不可见（"key"）通道（beatoraja 扩展）。
///
/// 此通道中的音符既不显示也不判定，但当玩家在正确时机按下对应按键时，
/// 其音频会被触发。
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct KeyChannel<'a> {
    /// 音频文件名。
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,
    /// 此通道中的不可见音符。
    pub notes: Vec<KeyNote>,
}

/// 单个不可见音符（beatoraja 扩展）。
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct KeyNote {
    /// 玩家通道。
    pub x: u64,
    /// 脉冲偏移。
    pub y: u64,
}

/// 将 `null` 反序列化为任意类型 `T` 的 [`Default::default()`]。
///
/// 通过 `#[serde(deserialize_with = "null_to_default")]` 用于 bmson 规范允许
/// `null` 但我们更倾向于使用更简单的 `Vec<T>` 类型的字段。
///
/// # Errors
///
/// 委托给 `T` 的 [`Deserialize`] 实现；如果 JSON 值既不是 `null` 也不是
/// 合法的 `T`，则返回错误。
#[inline]
pub fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// `judge_multiplier` / `life_multiplier` 的默认值（`1.00`）。
#[must_use]
#[inline]
pub const fn default_multiplier() -> f64 {
    1.00
}

/// 默认节拍分辨率（每四分音符 240 脉冲）。
#[must_use]
#[inline]
pub const fn default_resolution() -> u64 {
    240
}

/// 反序列化 `u64` 分辨率字段，将 `0` 替换为默认值 `240`。
///
/// 根据 bmson 规范，分辨率为 `0`、`null` 或 `undefined` 时必须视为 `240`。
/// 此辅助函数处理 `0` 的情况；`null`/`undefined` 由 `#[serde(default)]` 处理。
///
/// # Errors
///
/// 如果 JSON 值不是合法的无符号整数，则返回错误。
#[inline]
pub fn deserialize_resolution_nonzero<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<u64, D::Error> {
    let v = u64::deserialize(deserializer)?;
    if v == 0 { Ok(240) } else { Ok(v) }
}

/// 将 JSON 字符串反序列化为借用的 `&Path`。
///
/// JSON 输入必须是合法的 UTF-8 字符串；得到的 `&Path` 将相同字节重新解释
/// 为路径（零拷贝）。
///
/// # Errors
///
/// 如果 JSON 值不是字符串，则返回错误。
#[inline]
pub fn de_path<'de, D: Deserializer<'de>>(deserializer: D) -> Result<&'de Path, D::Error> {
    let s: &'de str = Deserialize::deserialize(deserializer)?;
    Ok(Path::new(s))
}

/// 将 JSON 字符串或 `null` 反序列化为 `Option<&Path>`。
///
/// JSON `null` 映射为 `None`；字符串映射为 `Some(&Path)`，将相同 UTF-8 字节
/// 重新解释为路径（零拷贝）。
///
/// # Errors
///
/// 如果 JSON 值既不是字符串也不是 `null`，则返回错误。
#[inline]
pub fn de_opt_path<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<&'de Path>, D::Error> {
    let s: Option<&'de str> = Deserialize::deserialize(deserializer)?;
    Ok(s.map(Path::new))
}
