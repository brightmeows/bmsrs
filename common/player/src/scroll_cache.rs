//! 预计算的滚动速度缓存。
//!
//! [`ScrollCache`] 在 [`Player`](crate::Player) 创建时由事件列表构建，
//! 提供 O(log n) 的滚动速度查询与累积位置查询，替代每次调用时的 O(n)
//! 线性扫描。

use bmsrs_chart::{CustomEvent, Event, EventKind, NoteExt};

/// 预计算的滚动速度段。
struct ScrollSegment {
    /// 本段起始的绝对脉冲。
    start_tick: u64,
    /// 本段起始处的累积滚动位置（以节拍为单位）。
    start_position: f64,
    /// 本段内的滚动倍率。
    rate: f64,
}

/// 预计算的滚动速度与位置缓存。
///
/// 存储已排序的速度段，每个段对应一次 [`EventKind::Scroll`] 变更。
/// 查询时通过二分查找定位生效段，将 O(n) 降为 O(log n)。
///
/// 位置计算将滚动速度对时间积分：`position += rate * delta_beats`，
/// 其中 `delta_beats = delta_ticks / resolution`。
pub struct ScrollCache {
    /// 按 `start_tick` 升序排列的速度段。
    segments: Vec<ScrollSegment>,
    /// 每四分音符的脉冲数。
    resolution: u64,
}

impl ScrollCache {
    /// 从已排序事件列表与节拍分辨率构建缓存。
    ///
    /// 仅遍历 `Scroll` 变体，跳过其他事件类型。
    /// 输入 `events` 必须按 [`Event::tick`] 升序排列（由谱面保证）。
    /// `resolution` 为每四分音符的脉冲数（通常为 240）。
    #[expect(
        clippy::cast_precision_loss,
        reason = "u64 tick/resolution offsets fit in f64"
    )]
    pub fn build<T: NoteExt, C: CustomEvent>(events: &[Event<T, C>], resolution: u64) -> Self {
        let raw_segments: Vec<(u64, f64)> = events
            .iter()
            .filter_map(|e| {
                if let EventKind::Scroll { rate } = &e.kind {
                    Some((e.tick(), *rate))
                } else {
                    None
                }
            })
            .collect();

        // 预计算每个段的累积位置。先加一个初始默认段（tick 0, rate 1.0）。
        let res_f = resolution as f64;
        let mut segments = vec![ScrollSegment {
            start_tick: 0,
            start_position: 0.0,
            rate: 1.0,
        }];
        let mut pos = 0.0f64;

        for &(tick, rate) in &raw_segments {
            let last_tick = segments.last().map_or(0, |s| s.start_tick);
            if tick > last_tick {
                let delta = (tick - last_tick) as f64 / res_f;
                pos = segments.last().map_or(0.0, |s| s.rate).mul_add(delta, pos);
            }
            segments.push(ScrollSegment {
                start_tick: tick,
                start_position: pos,
                rate,
            });
        }
        // 尾段：覆盖 tick 超出最后一个事件时的查询。非空事件列表下与最后
        // 一段等价（partition_point.saturating_sub(1) 回退到最后一段），
        // 但对空事件列表提供默认常量段（tick=0, rate=1.0）。
        segments.push(ScrollSegment {
            start_tick: raw_segments.last().map_or(0, |&(t, _)| t),
            start_position: pos,
            rate: raw_segments.last().map_or(1.0, |&(_, r)| r),
        });

        Self {
            segments,
            resolution,
        }
    }

    /// 返回 `tick` 处生效的滚动速度倍率。
    #[expect(
        clippy::indexing_slicing,
        reason = "first segment at tick 0 guarantees idx >= 1"
    )]
    pub fn rate_at(&self, tick: u64) -> f64 {
        let idx = self
            .segments
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1);
        self.segments[idx].rate
    }

    /// 返回 `tick` 处的累积滚动位置（以节拍为单位）。
    ///
    /// 位置 = 滚动速度对时间的积分。默认无 SCROLL 事件时，
    /// 位置 = `tick / resolution`（标准节拍对齐）。
    #[expect(
        clippy::indexing_slicing,
        clippy::cast_precision_loss,
        reason = "first segment at tick 0 guarantees idx >= 1; tick fits in f64"
    )]
    pub fn position_at(&self, tick: u64) -> f64 {
        let idx = self
            .segments
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1);
        let seg = &self.segments[idx];
        let delta = (tick - seg.start_tick) as f64 / self.resolution as f64;
        seg.rate.mul_add(delta, seg.start_position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RES: u64 = 240;

    fn make_scroll_events(pairs: &[(u64, f64)]) -> Vec<Event<(), bmsrs_chart::NoCustomEvent>> {
        pairs
            .iter()
            .map(|&(tick, rate)| Event::scroll(tick, rate))
            .collect()
    }

    #[test]
    fn no_scroll_events_returns_default() {
        let events: Vec<Event<(), bmsrs_chart::NoCustomEvent>> = vec![];
        let cache = ScrollCache::build(&events, RES);
        assert!((cache.rate_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.rate_at(480) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn single_scroll_event_affects_subsequent_ticks() {
        let events = make_scroll_events(&[(240, 2.0)]);
        let cache = ScrollCache::build(&events, RES);

        assert!((cache.rate_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.rate_at(239) - 1.0).abs() < 1e-9);
        assert!((cache.rate_at(240) - 2.0).abs() < 1e-9);
        assert!((cache.rate_at(480) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn multiple_scroll_events_take_last() {
        let events = make_scroll_events(&[(0, 1.5), (240, 0.5), (480, 2.0)]);
        let cache = ScrollCache::build(&events, RES);

        assert!((cache.rate_at(0) - 1.5).abs() < 1e-9);
        assert!((cache.rate_at(239) - 1.5).abs() < 1e-9);
        assert!((cache.rate_at(240) - 0.5).abs() < 1e-9);
        assert!((cache.rate_at(479) - 0.5).abs() < 1e-9);
        assert!((cache.rate_at(480) - 2.0).abs() < 1e-9);
        assert!((cache.rate_at(9999) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn matches_linear_scan_implementation() {
        let events = make_scroll_events(&[(120, 0.8), (360, 1.2), (600, 0.5)]);
        let cache = ScrollCache::build(&events, RES);

        for tick in [0u64, 60, 119, 120, 240, 360, 480, 600, 720, 1000] {
            let expected = linear_scroll_rate_at(&events, tick);
            let actual = cache.rate_at(tick);
            assert!(
                (actual - expected).abs() < 1e-9,
                "mismatch at tick {tick}: expected {expected}, got {actual}"
            );
        }
    }

    fn linear_scroll_rate_at(events: &[Event<(), bmsrs_chart::NoCustomEvent>], tick: u64) -> f64 {
        let end = events.partition_point(|e| e.tick() <= tick);
        let mut rate = 1.0;
        for event in &events[..end] {
            if let EventKind::Scroll { rate: sc_rate } = &event.kind {
                rate = *sc_rate;
            }
        }
        rate
    }

    // position_at tests

    #[test]
    fn position_at_no_events_tick_equals_resolution() {
        let events: Vec<Event<(), bmsrs_chart::NoCustomEvent>> = vec![];
        let cache = ScrollCache::build(&events, RES);
        assert!((cache.position_at(0) - 0.0).abs() < 1e-9);
        assert!((cache.position_at(240) - 1.0).abs() < 1e-9);
        assert!((cache.position_at(960) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn position_at_slow_scroll_lags_behind_tick() {
        // 0.5x SCROLL at tick 960 (= beat 4), tick 960-1440 (= beat 4-6)
        // position: 0-960 = 4 beats at 1.0 = 4.0
        //           960-1440 = 2 beats at 0.5 = 1.0 → cum = 5.0
        let events = make_scroll_events(&[(960, 0.5)]);
        let cache = ScrollCache::build(&events, RES);
        assert!((cache.position_at(960) - 4.0).abs() < 1e-9);
        assert!((cache.position_at(1440) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn position_at_fast_scroll_ahead_of_tick() {
        // 2.0x SCROLL at tick 480, tick 480-960 (= beat 2-4)
        // position: 0-480 = 2 beats at 1.0 = 2.0
        //           480-960 = 2 beats at 2.0 = 4.0 → cum = 6.0
        let events = make_scroll_events(&[(480, 2.0)]);
        let cache = ScrollCache::build(&events, RES);
        assert!((cache.position_at(480) - 2.0).abs() < 1e-9);
        assert!((cache.position_at(960) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn position_at_multiple_changes() {
        // 0→1.5, 240→0.5, 480→2.0
        // 0-240: 1 beat at 1.5 (scroll at tick 0) → 1.5
        // 240-480: 1 beat at 0.5 → 0.5, cum = 2.0
        // 480-960: 2 beats at 2.0 → 4.0, cum = 6.0
        let events = make_scroll_events(&[(0, 1.5), (240, 0.5), (480, 2.0)]);
        let cache = ScrollCache::build(&events, RES);
        assert!((cache.position_at(0) - 0.0).abs() < 1e-9);
        assert!((cache.position_at(240) - 1.5).abs() < 1e-9);
        assert!((cache.position_at(480) - 2.0).abs() < 1e-9);
        assert!((cache.position_at(960) - 6.0).abs() < 1e-9);
    }
}
