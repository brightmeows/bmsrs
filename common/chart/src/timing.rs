//! 用于脉冲 ↔ Duration 换算的计时轨。
//!
//! [`TimingTrack`] 持有初始 BPM、BPM 变更事件与停止事件。它提供
//! [`TimingTrack::tick_to_duration`] 与 [`TimingTrack::duration_to_tick`]
//! 用于在谱面位置（脉冲）与实际时间（[`Duration`]）之间换算。
//!
//! 内部计算使用 `f64` 运算（BPM 值本质上是浮点数）。`f64` ↔ [`Duration`]
//! 的转换仅发生在公开 API 边界处，通过 [`Duration::from_secs_f64`] 与
//! [`Duration::as_secs_f64`] 完成。

use std::time::Duration;

/// 用于将脉冲位置换算为实际时间的计时信息。
///
/// 所有事件都在绝对脉冲位置上。处理器负责将格式特有的位置
/// （BMSON 脉冲、BMS 小节）换算为脉冲。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimingTrack {
    /// 脉冲 0 处的初始 BPM。
    pub init_bpm: f64,
    /// BPM 变更事件，按脉冲升序排列。
    pub bpm_changes: Vec<BpmChange>,
    /// 停止（暂停）事件，按脉冲升序排列。
    pub stops: Vec<StopEvent>,
}

/// BPM 变更事件。
#[derive(Clone, Debug, PartialEq)]
pub struct BpmChange {
    /// BPM 发生变更的脉冲位置。
    pub tick: u64,
    /// 新的 BPM（每分钟拍数）。
    pub bpm: f64,
}

/// 停止（暂停）事件。
///
/// 当回放到达 `tick` 时，滚动暂停 `duration` 个脉冲的时长（按当前 BPM
/// 计算）。同一脉冲上的多个停止会累加。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopEvent {
    /// 停止开始的脉冲位置。
    pub tick: u64,
    /// 时长（脉冲数）。
    pub duration: u64,
}

/// 合并时间线的内部事件表示。
#[derive(Clone, Copy)]
enum TimingEvent {
    /// BPM 变更 —— `is_stop = false` 确保同一脉冲上 BPM 排在 Stop 之前。
    Bpm(f64),
    /// 停止，时长以脉冲数表示。
    Stop(u64),
}

impl TimingEvent {
    /// 当此事件为 [`TimingEvent::Stop`] 时返回 `true`。
    const fn is_stop(self) -> bool {
        matches!(self, Self::Stop(_))
    }
}

impl TimingTrack {
    /// 由 `bpm_changes` 与停止事件构造已排序的事件列表。
    ///
    /// 同一脉冲上，BPM 变更排在停止之前（依据 BMSON 规范：
    /// "speed will first change, then the music pauses"）。
    fn build_events(&self) -> Vec<(u64, TimingEvent)> {
        let mut events: Vec<(u64, TimingEvent)> = Vec::new();
        for bc in &self.bpm_changes {
            events.push((bc.tick, TimingEvent::Bpm(bc.bpm)));
        }
        for st in &self.stops {
            events.push((st.tick, TimingEvent::Stop(st.duration)));
        }
        // 按脉冲排序，再按 BPM（false）排在 Stop（true）之前。
        events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.is_stop().cmp(&b.1.is_stop())));
        events
    }

    /// 将脉冲位置换算为实际时间 [`Duration`]。
    ///
    /// 在带有停止事件的脉冲上，返回的时间是暂停**之前**的时刻
    /// （依据 BMSON 规范，该脉冲上的音符在暂停之前激活）。
    /// 严格位于目标脉冲之前的停止贡献其完整暂停时长。
    ///
    /// # Panic（仅 debug 构建）
    ///
    /// debug 构建中断言 `resolution > 0` 与 `init_bpm > 0`。
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "larger function; compiler can decide inlining"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "lossless or explicitly-rounded numeric conversion"
    )]
    pub fn tick_to_duration(&self, tick: u64, resolution: u64) -> Duration {
        debug_assert!(resolution > 0, "resolution must be > 0");
        debug_assert!(self.init_bpm > 0.0, "init_bpm must be > 0");

        let res = resolution as f64;
        let events = self.build_events();

        let mut seconds = 0.0f64;
        let mut current_tick = 0u64;
        let mut current_bpm = self.init_bpm;

        for (event_tick, event) in &events {
            if *event_tick > tick {
                break;
            }
            // 将回放推进到 event_tick。
            if *event_tick > current_tick {
                let delta = (*event_tick - current_tick) as f64;
                seconds += delta / res * 60.0 / current_bpm;
                current_tick = *event_tick;
            }
            match event {
                TimingEvent::Bpm(bpm) => {
                    current_bpm = *bpm;
                }
                TimingEvent::Stop(duration) => {
                    // 仅当停止严格位于目标脉冲之前时才计入停止时间。
                    // 在目标脉冲本身处，时间是暂停之前的时刻（依据规范）。
                    if *event_tick < tick {
                        seconds += *duration as f64 / res * 60.0 / current_bpm;
                    }
                }
            }
        }

        // 从最后一个事件到目标脉冲的剩余时间。
        if tick > current_tick {
            let delta = (tick - current_tick) as f64;
            seconds += delta / res * 60.0 / current_bpm;
        }

        Duration::from_secs_f64(seconds)
    }

    /// 将实际时间 [`Duration`] 换算为最近的脉冲位置。
    ///
    /// 这是 [`tick_to_duration`](Self::tick_to_duration) 的逆运算。
    /// 停止中消耗的时间不会推进脉冲。
    ///
    /// # Panic（仅 debug 构建）
    ///
    /// debug 构建中断言 `resolution > 0` 与 `init_bpm > 0`。
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded result is within u64 range"
    )]
    #[expect(clippy::cast_sign_loss, reason = "remaining time is non-negative")]
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "larger function; compiler can decide inlining"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "lossless or explicitly-rounded numeric conversion"
    )]
    pub fn duration_to_tick(&self, duration: Duration, resolution: u64) -> u64 {
        debug_assert!(resolution > 0, "resolution must be > 0");
        debug_assert!(self.init_bpm > 0.0, "init_bpm must be > 0");

        let seconds = duration.as_secs_f64();

        if seconds <= 0.0 {
            return 0;
        }

        let res = resolution as f64;
        let events = self.build_events();

        let mut remaining = seconds;
        let mut current_tick = 0u64;
        let mut current_bpm = self.init_bpm;

        for (event_tick, event) in &events {
            // 将回放推进到 event_tick。
            if *event_tick > current_tick {
                let delta_ticks = *event_tick - current_tick;
                let delta_seconds = delta_ticks as f64 / res * 60.0 / current_bpm;
                if delta_seconds >= remaining {
                    return current_tick + (remaining * res * current_bpm / 60.0).round() as u64;
                }
                remaining -= delta_seconds;
                current_tick = *event_tick;
            }

            match event {
                TimingEvent::Bpm(bpm) => {
                    current_bpm = *bpm;
                }
                TimingEvent::Stop(stop_duration) => {
                    let stop_seconds = *stop_duration as f64 / res * 60.0 / current_bpm;
                    if stop_seconds >= remaining {
                        // 目标位于停止内 —— 脉冲不推进。
                        return current_tick;
                    }
                    remaining -= stop_seconds;
                }
            }
        }

        // 目标超出所有事件。
        current_tick + (remaining * res * current_bpm / 60.0).round() as u64
    }
}
