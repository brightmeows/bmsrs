//! 音频通道切片算法。
//!
//! 每个 [`SoundChannel`](bmson_def::SoundChannel) 将一个音频文件与其全部
//! [`NoteEvent`] 打包在一起。处理器预先计算音频切片，使播放器在运行时
//! 只需按索引查表即可拿到预设好长度的 [`AudioAsset`]。
//!
//! # 算法
//!
//! 1. 从所有音符事件中收集唯一的脉冲（`y`）位置。
//! 2. 升序排序。
//! 3. 对每个脉冲，判断它是否为**重启点**：若该脉冲上*任意*音符的
//!    `c: false`，则视为重启（同一脉冲上 `c` 标志混合 → 视为重启）。
//! 4. 通过 [`TimingTrack`] 将每个脉冲换算为谱面时间。
//! 5. 为每个脉冲 *P<sub>i</sub>* 创建一个 [`AudioAsset`]，其 `start`
//!    取决于延续标志：
//!    - **重启**（`c: false`）：`start = 0`（从文件开头播放）。
//!    - **延续**（`c: true`）：`start` = *P<sub>i</sub>* 处的谱面时间
//!      减去最近一次重启点处的谱面时间（从音频在不重启情况下的应有位置
//!      开始播放）。
//! 6. `duration` 为到下一个脉冲的谱面时间差；最后一片为 `None`。

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use bmson_def::SoundChannel;
use bmsrs_chart::{AudioAsset, TimingCache};

/// 对单条音频通道切片的输出。
///
/// `pulse_to_index` 将音符事件脉冲映射到其 [`AudioAsset`] 在本切片
/// `assets` 向量中的索引。
pub struct SlicedChannel {
    /// 音频素材，按脉冲升序排列。
    pub assets: Vec<AudioAsset>,
    /// 脉冲 → 音频素材索引的查找表。
    pub pulse_to_index: BTreeMap<u64, usize>,
}

/// 将一条音频通道切片为预计算的 [`AudioAsset`]。
///
/// 算法详见[模块文档](self)。`timing` 为由 [`TimingTrack`] 预计算的
/// [`TimingCache`]，提供 O(log n) 的脉冲→时间换算（相比直接调用
/// [`TimingTrack::tick_to_duration`] 的 O(n) 扫描，在脉冲密集的通道上
/// 将整体复杂度从 O(N·M) 降为 O(M·log N)）。
///
/// [`TimingTrack`]: bmsrs_chart::TimingTrack
/// [`TimingTrack::tick_to_duration`]: bmsrs_chart::TimingTrack::tick_to_duration
pub fn slice_channel(channel: &SoundChannel<'_>, timing: &TimingCache) -> SlicedChannel {
    // 1. 收集唯一的脉冲位置。BTreeSet 迭代本就升序，无需再次排序。
    let pulses: Vec<u64> = channel
        .note_events
        .iter()
        .map(|n| n.y)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // 2. 标记重启点：即 *任意* 音符 c:false 的脉冲。
    //    （按规范，同一脉冲上 c 标志混合 → 视为重启。）
    let restart_pulses: BTreeSet<u64> = channel
        .note_events
        .iter()
        .filter(|n| !n.c)
        .map(|n| n.y)
        .collect();

    // 3. 将每个脉冲换算为谱面时间。
    let pulse_times: Vec<(u64, Duration)> = pulses
        .iter()
        .map(|&p| (p, timing.tick_to_duration(p)))
        .collect();

    // 4. 构建带有正确音频起始偏移的 AudioAsset。
    let mut assets = Vec::with_capacity(pulse_times.len());
    let mut pulse_to_index = BTreeMap::new();
    // 共享路径：同一通道所有切片引用同一个 Arc，仅增加引用计数。
    let shared_path = Arc::from(channel.name.to_path_buf());
    // 记录最近一次重启点处的谱面时间。
    let mut last_restart_time = Duration::ZERO;

    for (i, &(pulse, chart_time)) in pulse_times.iter().enumerate() {
        // 首个脉冲始终视为重启（此前没有可延续的音频上下文）。
        let is_restart = i == 0 || restart_pulses.contains(&pulse);

        let audio_start = if is_restart {
            Duration::ZERO
        } else {
            chart_time.saturating_sub(last_restart_time)
        };

        if is_restart {
            last_restart_time = chart_time;
        }

        let duration = pulse_times
            .get(i + 1)
            .map(|&(_, next_time)| next_time.saturating_sub(chart_time));

        let asset = AudioAsset {
            path: Arc::clone(&shared_path),
            start: audio_start,
            duration,
        };
        pulse_to_index.insert(pulse, assets.len());
        assets.push(asset);
    }

    SlicedChannel {
        assets,
        pulse_to_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmson_def::NoteEvent;
    use bmsrs_chart::TimingTrack;
    use std::path::Path;

    const RES: u64 = 240;

    fn timing_120() -> TimingCache {
        TimingCache::new(&TimingTrack::new(120.0, vec![], vec![]), RES)
    }

    #[test]
    fn three_unique_pulses_produce_three_assets() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 2,
                    y: 240,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 480,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120());

        assert_eq!(result.assets.len(), 3);
        assert_eq!(result.pulse_to_index.len(), 3);
        // 脉冲 0：首个脉冲 → 重启 → start=0
        assert!(result.assets[0].start.is_zero());
        assert_eq!(result.assets[0].duration, Some(Duration::from_millis(500)));
        // 脉冲 240：c:true → 从最近重启点（0）延续 → start = chart(240) - chart(0) = 0.5s
        assert_eq!(result.assets[1].start, Duration::from_millis(500));
        assert_eq!(result.assets[1].duration, Some(Duration::from_millis(500)));
        // 脉冲 480：c:false → 重启 → start=0
        assert!(result.assets[2].start.is_zero());
        assert!(result.assets[2].duration.is_none());
    }

    #[test]
    fn duplicate_pulse_deduplicated() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 2,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120());

        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.pulse_to_index.get(&240), Some(&0));
        // 首个脉冲 → 重启 → start=0
        assert!(result.assets[0].start.is_zero());
    }

    #[test]
    fn first_slice_starts_at_zero() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![NoteEvent {
                x: 1,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            }],
        };

        let result = slice_channel(&channel, &timing_120());

        assert!(result.assets[0].start.is_zero());
        assert!(result.assets[0].duration.is_none());
    }

    #[test]
    fn intermediate_slice_duration_is_gap() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120());

        // 120 BPM、节拍分辨率 240：脉冲 0→0s，脉冲 240→0.5s
        assert!(result.assets[0].start.is_zero());
        let d0 = result.assets[0].duration.expect("first slice has duration");
        assert_eq!(d0, Duration::from_millis(500));
        // 第二个脉冲为重启 → start=0
        assert!(result.assets[1].start.is_zero());
    }

    #[test]
    fn last_slice_duration_is_none() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 480,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120());

        assert!(result.assets.last().is_some_and(|a| a.duration.is_none()));
    }

    #[test]
    fn continuation_pulse_offsets_from_last_restart() {
        // 脉冲序列：0（c:false，重启）、240（c:true，延续）、
        // 480（c:true，延续）、720（c:false，重启）、840（c:true，延续）。
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 480,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 720,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 840,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120());

        // 5 个唯一脉冲 → 5 个音频素材
        assert_eq!(result.assets.len(), 5);

        // 素材 0（脉冲 0，重启）：start=0
        assert!(result.assets[0].start.is_zero());
        // 素材 1（脉冲 240，c:true）：相对最近重启点（0）的偏移 → 0.5s
        assert_eq!(result.assets[1].start, Duration::from_millis(500));
        // 素材 2（脉冲 480，c:true）：相对最近重启点（0）的偏移 → 1.0s
        assert_eq!(result.assets[2].start, Duration::from_secs(1));
        // 素材 3（脉冲 720，重启）：start=0
        assert!(result.assets[3].start.is_zero());
        // 素材 4（脉冲 840，c:true）：相对最近重启点（720）的偏移 → chart(840) - chart(720) = 0.25s
        assert_eq!(result.assets[4].start, Duration::from_millis(250));
    }

    #[test]
    fn mixed_c_at_same_pulse_treated_as_restart() {
        // 脉冲 240 上有两个音符：一个 c:true，一个 c:false → 视为重启。
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 2,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120());
        // 首个脉冲 → 始终视为重启
        assert!(result.assets[0].start.is_zero());
    }
}
