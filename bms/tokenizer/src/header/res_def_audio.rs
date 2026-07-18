//! 音频资源定义头部：`#WAV`、`#EXWAV`、`#WAVCMD`、
//! `#CDDA`、`#MIDIFILE`、`#PATH_WAV`。

use std::fmt;

use crate::index::WavIndex;
use crate::{BmsHeader, BmsTokenAttr, BmsTryFromError, BmsValue};

/// `#EXWAV{id}` 的扩展音频效果参数（nanasi 扩展）。
///
/// 未指定的参数为 `None`——播放器应用各自默认值。
///
/// 参数范围（BMS 规范定义整数）：
/// - **pan**（`p`）：`[-10000, 10000]`，默认 `0`。左端 -10000，右端 10000。
/// - **volume**（`v`）：`[-10000, 0]`，默认 `0`（原始音量）。
///   降低一个声道的音量而非提升另一声道。
/// - **frequency**（`f`）：`[100, 100000]` Hz（音高控制）。
///
/// `#EXWAV` 索引与 `#WAV` 共享同一命名空间。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExWavParams<C> {
    /// 声像（`p` flag）。范围：`[-10000, 10000]`，`0` = 居中。
    pub pan: Option<i32>,
    /// 音量衰减（`v` flag）。范围：`[-10000, 0]`，`0` = 原声。
    pub volume: Option<i32>,
    /// 频率（`f` flag）。范围：`[100, 100000]` Hz。
    pub frequency: Option<u32>,
    /// 资源文件路径或名称。
    pub filename: C,
}

impl<C: AsRef<str> + fmt::Display> fmt::Display for ExWavParams<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 按 pan→volume→frequency 固定顺序输出存在的 flag。
        let mut flags = String::new();
        let mut values: Vec<String> = Vec::new();
        if let Some(p) = self.pan {
            flags.push('p');
            values.push(p.to_string());
        }
        if let Some(v) = self.volume {
            flags.push('v');
            values.push(v.to_string());
        }
        if let Some(fr) = self.frequency {
            flags.push('f');
            values.push(fr.to_string());
        }
        if flags.is_empty() {
            write!(f, "{}", self.filename)
        } else {
            write!(f, "{flags}")?;
            for v in &values {
                write!(f, " {v}")?;
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

        let is_flags = !first.is_empty()
            && first.chars().all(|c| matches!(c, 'p' | 'v' | 'f'))
            && first.len() <= 3;

        if is_flags {
            let mut pan = None;
            let mut volume = None;
            let mut frequency = None;
            // 跟踪已消费字节，保留零拷贝路径（filename 切片借用 `s`）。
            let mut consumed = first.len();
            for flag in first.chars() {
                // 跳过引导空白，定位下一个数值起点。
                while s
                    .as_bytes()
                    .get(consumed)
                    .is_some_and(u8::is_ascii_whitespace)
                {
                    consumed += 1;
                }
                let val_start = consumed;
                while s
                    .as_bytes()
                    .get(consumed)
                    .is_some_and(|b| !b.is_ascii_whitespace())
                {
                    consumed += 1;
                }
                let val_str = s.get(val_start..consumed)?;
                let val: i64 = val_str.parse().ok()?;
                match flag {
                    'p' => {
                        let v: i32 = val.try_into().ok()?;
                        if !(-10000..=10000).contains(&v) {
                            return None;
                        }
                        pan = Some(v);
                    }
                    'v' => {
                        let v: i32 = val.try_into().ok()?;
                        if !(-10000..=0).contains(&v) {
                            return None;
                        }
                        volume = Some(v);
                    }
                    'f' => {
                        let v: u32 = val.try_into().ok()?;
                        if !(100..=100_000).contains(&v) {
                            return None;
                        }
                        frequency = Some(v);
                    }
                    _ => return None,
                }
            }
            // filename 是最后一个值之后的剩余部分（可能含空格）。
            while s
                .as_bytes()
                .get(consumed)
                .is_some_and(u8::is_ascii_whitespace)
            {
                consumed += 1;
            }
            let filename = s.get(consumed..).filter(|f| !f.is_empty())?;
            Some(Self {
                pan,
                volume,
                frequency,
                filename: C::from(filename),
            })
        } else {
            // 无 flags：整体为 filename。
            Some(Self {
                pan: None,
                volume: None,
                frequency: None,
                filename: C::from(s.trim()),
            })
        }
    }
}

/// `#WAVCMD` 的命令种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavCmdKind {
    /// `00`——音高（MIDI 音符号，基准 60 = 中央 C）。范围：`[0, 127]`。
    Pitch,
    /// `01`——音量（百分比，100 = 原始）。
    Volume,
    /// `02`——再生时长（半毫秒单位）。
    Time,
}

impl std::fmt::Display for WavCmdKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pitch => "00",
            Self::Volume => "01",
            Self::Time => "02",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for WavCmdKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "00" => Ok(Self::Pitch),
            "01" => Ok(Self::Volume),
            "02" => Ok(Self::Time),
            _ => Err(()),
        }
    }
}

/// `#WAVCMD` 的解析参数——音高/音量/时长覆盖（MacBeat 扩展）。
///
/// 格式：`commandID wavIndex value`。命令：`00` = 音高、
/// `01` = 音量、`02` = 时长。仅 `MacBeat` 处理这些；
/// Sonorous 解析但忽略。
///
/// 解析失败时回退到 [`BmsHeaderFallback`](crate::BmsHeaderFallback)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavCmdParams {
    /// 命令种类。
    pub command: WavCmdKind,
    /// 目标 WAV 索引。
    pub wav_index: WavIndex,
    /// 参数值（非负整数，语义由 `command` 决定）。
    ///
    /// - `Pitch`：MIDI 音符号，范围 `[0, 127]`。
    /// - `Volume`：百分比，`100` = 原始。大于 `100` 的值可能导致削波。
    /// - `Time`：半毫秒单位。小于 `50` 的值可能不可靠。
    pub value: u32,
}

impl std::fmt::Display for WavCmdParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.command, self.wav_index, self.value)
    }
}

impl std::str::FromStr for WavCmdParams {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = (|| {
            let mut parts = s.split_whitespace();
            let command: WavCmdKind = parts.next()?.parse().ok()?;
            let wav_index: WavIndex = parts.next()?.parse().ok()?;
            let value: u32 = parts.next()?.parse().ok()?;
            if parts.next().is_some() {
                return None;
            }
            // 音高命令（00）的值范围：MIDI 音符号 [0, 127]。
            if command == WavCmdKind::Pitch && value > 127 {
                return None;
            }
            Some(Self {
                command,
                wav_index,
                value,
            })
        })();
        parsed.ok_or(())
    }
}

/// 音频资源定义头部。
///
/// 这些命令定义谱面使用的音频文件。WAV 与 OGG 是
/// 受支持最广的格式；MP3 在大多数播放器中引入可感知的延迟，
/// 通常避免使用。
#[derive(Debug, Clone, PartialEq, Eq, BmsTokenAttr)]
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
        params: WavCmdParams,
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
