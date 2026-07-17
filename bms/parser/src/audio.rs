//! 音频资源定义。
//!
//! 对应分词器的 [`BmsHeaderResDefAudio`]。

use std::collections::BTreeMap;

use bms_tokenizer::{BmsBase, BmsHeaderResDefAudio, ExWavParams, WavCmdParams, WavIndex};

/// `#EXWAV` 的 owned 参数（tokenizer 的 `ExWavParams` 的 owned 对应）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedExWavParams {
    /// 声像。
    pub pan: Option<i32>,
    /// 音量衰减。
    pub volume: Option<i32>,
    /// 频率。
    pub frequency: Option<u32>,
    /// 文件名。
    pub filename: String,
}

impl<C: AsRef<str>> From<&ExWavParams<C>> for OwnedExWavParams {
    fn from(p: &ExWavParams<C>) -> Self {
        Self {
            pan: p.pan,
            volume: p.volume,
            frequency: p.frequency,
            filename: p.filename.as_ref().to_owned(),
        }
    }
}

/// 音频资源定义。
///
/// 索引定义使用 `BTreeMap`；标量字段使用 `Option`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Audio {
    /// 音效 / BGM 文件定义（`#WAV`、`#EXWAV`）。
    pub wav_files: BTreeMap<WavIndex, String>,
    /// 按索引存储的 `#EXWAV` 效果参数（pan/volume/frequency）。
    pub ex_wav_params: BTreeMap<WavIndex, OwnedExWavParams>,
    /// 音频播放命令（`#WAVCMD`）。
    pub wav_cmd: Option<WavCmdParams>,
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
                self.ex_wav_params
                    .insert(nid, OwnedExWavParams::from(params));
            }
            BmsHeaderResDefAudio::WavCmd { params } => {
                self.wav_cmd = Some(params.clone());
            }
            BmsHeaderResDefAudio::Cdda(s) => self.cdda = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::Midifile(s) => self.midifile = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::PathWav(s) => self.path_wav = Some(s.as_ref().to_owned()),
        }
    }
}
