//! 音频资源定义头部：`#WAV`、`#EXWAV`、`#WAVCMD`、
//! `#CDDA`、`#MIDIFILE`、`#PATH_WAV`。

use std::fmt;

use crate::index::WavIndex;
use crate::{BmsHeader, BmsTokenAttr, BmsTryFromError, BmsValue};

/// `#EXWAV{id}` 的参数——带声相/音量/频率控制的扩展 WAV
/// （nanasi 扩展）。
///
/// `flags` 是由 `{p, v, f}` 中的字符组成的字符串，指示
/// 后续的音频参数。值的数量等于 flag 字符数。flag 顺序决定
/// 值的顺序：
///
/// ```text
/// #EXWAV01 vfp -50 100 -10000 sound.wav
///           │      │   │    │        └─ filename
///           │      │   │    └─ frequency (f): 100 Hz
///           │      │   └─ volume (v): -50 (attenuation)
///           │      └─ pan (p): -10000 (hard left)
///           └─ flags: volume, frequency, pan
/// ```
///
/// 参数范围：
/// - **pan**（`p`）：`-10000` 到 `10000`（左 ↔ 右；默认 `0`）。
///   降低一个声道的音量而非提升另一声道。
/// - **volume**（`v`）：`-10000` 到 `0`（衰减；`0` = 原始）。
/// - **frequency**（`f`）：`100` 到 `100000` Hz（音高控制）。
///
/// `#EXWAV` 索引与 `#WAV` 共享同一命名空间。
#[derive(Debug, Clone, PartialEq)]
pub struct ExWavParams<C> {
    /// flag 字符（例如 `"pvf"`）。
    pub flags: C,
    /// 解析出的数值，每个 flag 字符一个值。
    pub values: Vec<f64>,
    /// 资源文件路径或名称。
    pub filename: C,
}

impl<C: AsRef<str> + fmt::Display> fmt::Display for ExWavParams<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.flags.as_ref().is_empty() {
            write!(f, "{}", self.filename)
        } else {
            write!(f, "{}", self.flags)?;
            for val in &self.values {
                write!(f, " {val}")?;
            }
            write!(f, " {}", self.filename)
        }
    }
}

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C>
    for ExWavParams<C>
{
    fn parse(s: &'a str) -> Option<Self> {
        let mut parts = s.split_whitespace();
        let first = parts.next()?;

        let is_flags = !first.is_empty() && first.chars().all(|c| matches!(c, 'p' | 'v' | 'f'));

        if is_flags {
            let flag_count = first.len();
            // 在 flags 之后收集恰好 `flag_count` 个数值。
            let values: Vec<f64> = parts
                .take(flag_count)
                .map(|p| p.parse().ok())
                .collect::<Option<_>>()?;
            if values.len() != flag_count {
                return None;
            }
            let filename = nth_whitespace_field_rest(s, flag_count + 1);
            Some(Self {
                flags: C::from(first),
                values,
                filename: C::from(filename),
            })
        } else {
            Some(Self {
                flags: C::from(""),
                values: Vec::new(),
                filename: C::from(s.trim()),
            })
        }
    }
}

/// 返回 `s` 中从第 N 个空白分隔字段（0 起索引）开始的子串，
/// 去除引导空白。
fn nth_whitespace_field_rest(s: &str, n: usize) -> &str {
    let mut start = 0;
    let mut field = 0;
    let bytes = s.as_bytes();
    while start < bytes.len() && field < n {
        while start < bytes.len() && bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
            start += 1;
        }
        if start >= bytes.len() {
            break;
        }
        while start < bytes.len() && bytes.get(start).is_some_and(|b| !b.is_ascii_whitespace()) {
            start += 1;
        }
        field += 1;
    }
    while start < bytes.len() && bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    s.get(start..).unwrap_or("")
}

/// `#WAVCMD` 的参数——音高/音量/时长覆盖（MacBeat 扩展）。
///
/// `MacBeat` 独占的伪 MOD 音效命令。使用此命令的 BMS 文件须将扩展名
/// 改为 `.mbm`（`MacBeat` MOD），其他播放器通常不支持。
///
/// # 格式
///
/// `#WAVCMD <commandID> <wavIndex> <value>`
///
/// - `commandID`：2 字符命令标识（见下表）
/// - `wavIndex`：16 进制，对应 `#WAVxx` 定义的索引
/// - `value`：**十进制**非负整数，语义取决于 `commandID`
///
/// # 命令表
///
/// | `commandID` | 功能 | `value` 语义 | 范围/单位 |
/// |-------------|------|-------------|----------|
/// | `00` | 音高 | MIDI 风格音符号，基准 `60` = 中央 C | `0`–`127` |
/// | `01` | 音量 | 百分比 | `100` = 原始音量；可超 `100` 但可能爆音 |
/// | `02` | 再生时长 | 半毫秒单位（秒 × 2000） | `50`ms 以下截断为 `0` |
///
/// 未使用 `#WAVCMD` 时的默认值：音高 `60`、音量 `100`、时长 `0`（播放到结束）。
///
/// `#WAVCMD` 允许同命令多次出现，每行独立应用到一个 `wavIndex`。
///
/// 来源：[MacBeat mbm.txt](http://harinezumi.s14.xrea.com/download/mbm.txt)、
/// [BMS command memo](https://hitkey.nekokan.dyndns.info/cmds.htm)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavCmdParams<C> {
    /// 命令 ID（`00`、`01`、`02`）。
    pub command_id: C,
    /// 目标 WAV 索引。
    pub wav_index: C,
    /// 参数值（非负整数，语义由 `command_id` 决定）。
    pub value: u32,
}

impl<C: AsRef<str> + fmt::Display> fmt::Display for WavCmdParams<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.command_id, self.wav_index, self.value)
    }
}

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C>
    for WavCmdParams<C>
{
    fn parse(s: &'a str) -> Option<Self> {
        let mut parts = s.split_whitespace();
        let command_id = parts.next()?;
        let wav_index = parts.next()?;
        let value: u32 = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            command_id: C::from(command_id),
            wav_index: C::from(wav_index),
            value,
        })
    }
}

/// 音频资源定义头部。
///
/// 这些命令定义谱面使用的音频文件。WAV 与 OGG 是
/// 受支持最广的格式；MP3 在大多数播放器中引入可感知的延迟，
/// 通常避免使用。
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefAudio<C> {
    /// `#WAV{id}`——音效或 BGM 文件定义。
    ///
    /// 被通道 `#xxx01`（BGM）、`#xxx11-19` / `#xxx21-29`
    /// （可见音符）、`#xxx31-39` / `#xxx41-49`（不可见音符）、
    /// `#xxx51-69`（长音）、`#xxxD1-E9`（地雷）等引用。
    ///
    /// **多重定义技巧**：将同一文件分配给多个
    /// `#WAV` 索引可增加复音数。例如 `#WAV01 kick.wav` 和
    /// `#WAV02 kick.wav` 允许同一声音同时播放两次，
    /// 而不会互相打断。
    ///
    /// `#WAV00` 较特殊：它定义地雷爆炸音
    /// （被通道 `#xxxD1-E9` 引用）。
    #[bms_token("#WAV{id} {filename}")]
    Wav {
        /// 2 字符索引（例如 `"01"`、`"2A"`）。
        id: WavIndex,
        /// 资源文件路径或名称。
        filename: C,
    },
    /// `#EXWAV{id}`——带声相/音量/频率控制的扩展 WAV（nanasi）。
    ///
    /// 共享 `#WAV` 索引命名空间。仅 nanasi 处理其
    /// 效果；其他播放器回退为原样播放文件。
    #[bms_token("#EXWAV{id} {params}")]
    #[bms_fallback]
    ExWav {
        /// 2 字符索引。
        id: WavIndex,
        /// 解析出的 EXWAV 参数。
        params: ExWavParams<C>,
    },
    /// `#WAVCMD`——音高/音量/时长覆盖（`MacBeat` 扩展）。
    ///
    /// 格式：`commandID wavIndex value`。命令：`00` = 音高、
    /// `01` = 音量、`02` = 时长。仅 `MacBeat` 处理这些；
    /// Sonorous 解析但忽略。
    ///
    /// 解析失败时回退到 [`BmsHeaderFallback`](crate::BmsHeaderFallback)。
    #[bms_token("#WAVCMD {params}")]
    #[bms_fallback]
    WavCmd {
        /// 解析出的 WAVCMD 参数。
        params: WavCmdParams<C>,
    },
    /// `#CDDA`——以 CD-DA 音轨作为 BGM（仅 DDR）。
    ///
    /// 指定一个 CD 音轨号作为背景音乐播放。
    #[bms_token("#CDDA {}")]
    Cdda(C),
    /// `#MIDIFILE`——以 MIDI 文件作为 BGM（BM98 起源）。
    ///
    /// 依赖硬件且有可感知延迟。不推荐用于
    /// 新谱面。受 BM98、DDR、`IIDXv`、HDX、Sonorous 支持。
    #[bms_token("#MIDIFILE {}")]
    Midifile(C),
    /// `#PATH_WAV`——音频文件查找的目录前缀（BMEV 起源）。
    ///
    /// 存在时，`#WAV` 文件名相对于此
    /// 目录解析。**分发前应注释掉**以
    /// 避免在其他系统上的路径问题。
    #[bms_token("#PATH_WAV {}")]
    PathWav(C),
}

// From / TryFrom 转换

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderResDefAudio<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::ResDefAudio(a) => Ok(a),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}
