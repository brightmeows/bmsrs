//! 音频资源定义。
//!
//! 对应分词器的 [`BmsHeaderResDefAudio`]。

use std::collections::BTreeMap;

use bms_tokenizer::{BmsBase, BmsHeaderResDefAudio, WavIndex};

/// `#EXWAV` 的扩展音频效果参数。
///
/// 每个标志字符（`p`/`v`/`f`）都有一个对应的数值：
/// - **pan（声像）**（`p`）：`-10000` 到 `10000`，默认 `0`。
/// - **volume（音量）**（`v`）：`-10000` 到 `0`，默认 `0`（原始音量）。
/// - **frequency（频率）**（`f`）：`100` 到 `100000` Hz。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExWavParams {
    /// 标志字符（例如 `"pvf"`）。
    pub flags: String,
    /// 数值，按标志字符的顺序一一对应。
    pub values: Vec<f64>,
}

/// 音频资源定义。
///
/// 索引定义使用 `BTreeMap`；标量字段使用 `Option`。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Audio {
    /// 音效 / BGM 文件定义（`#WAV`、`#EXWAV`）。
    pub wav_files: BTreeMap<WavIndex, String>,
    /// 按索引存储的 `#EXWAV` 效果参数（pvf/pan/vol/freq）。
    pub ex_wav_params: BTreeMap<WavIndex, ExWavParams>,
    /// 音频播放命令（`#WAVCMD`）。
    pub wav_cmd: Option<String>,
    /// CD 音轨引用（`#CDDA`）。
    pub cdda: Option<String>,
    /// MIDI 文件引用（`#MIDIFILE`）。
    pub midifile: Option<String>,
    /// 音频文件查找的目录前缀（`#PATH_WAV`）。
    pub path_wav: Option<String>,
}

impl Audio {
    /// 将一个音频资源头部命令应用到此结构体。
    ///
    /// 索引键（`WavIndex`）使用 `base` 归一化，以便在标准（Base36）
    /// BMS 文件中进行不区分大小写的比较。
    pub fn apply<C: AsRef<str>>(&mut self, header: &BmsHeaderResDefAudio<C>, base: BmsBase) {
        match header {
            BmsHeaderResDefAudio::Wav { id, filename } => {
                self.wav_files.insert(
                    WavIndex::from(id.normalize(base)),
                    filename.as_ref().to_owned(),
                );
            }
            BmsHeaderResDefAudio::ExWav { id, params } => {
                let nid = WavIndex::from(id.normalize(base));
                self.wav_files
                    .insert(nid, params.filename.as_ref().to_owned());
                self.ex_wav_params.insert(
                    nid,
                    ExWavParams {
                        flags: params.flags.as_ref().to_owned(),
                        values: params.values.clone(),
                    },
                );
            }
            BmsHeaderResDefAudio::WavCmd(s) => self.wav_cmd = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::Cdda(s) => self.cdda = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::Midifile(s) => self.midifile = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::PathWav(s) => self.path_wav = Some(s.as_ref().to_owned()),
        }
    }
}
