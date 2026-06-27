//! 预计算的滚动速度缓存。
//!
//! [`ScrollCache`] 在 [`Player`](crate::Player) 创建时由事件列表构建，
//! 提供 O(log n) 的滚动速度查询，替代每次调用时的 O(n) 线性扫描。

use bmsrs_chart::{CustomEvent, Event, NoteExt};

/// 预计算的滚动速度缓存。
///
/// 存储已排序的 `(start_tick, rate)` 段，每个段对应一次 [`Event::Scroll`]
/// 变更。查询时通过二分查找定位生效段，将 O(n) 降为 O(log n)。
pub struct ScrollCache {
    /// 按 `start_tick` 升序排列的 (段起始脉冲, 滚动倍率) 对。
    segments: Vec<(u64, f64)>,
}

impl ScrollCache {
    /// 从已排序事件列表构建缓存。
    ///
    /// 仅遍历 `Scroll` 变体，跳过其他事件类型。
    /// 输入 `events` 必须按 [`Event::tick`] 升序排列（由谱面保证）。
    pub fn build<T: NoteExt, C: CustomEvent>(events: &[Event<T, C>]) -> Self {
        let segments: Vec<(u64, f64)> = events
            .iter()
            .filter_map(|e| {
                if let Event::Scroll { tick, rate } = e {
                    Some((*tick, *rate))
                } else {
                    None
                }
            })
            .collect();
        Self { segments }
    }

    /// 返回 `tick` 处生效的滚动速度倍率。
    ///
    /// 使用二分查找定位最后一个不晚于 `tick` 的变更段。
    /// 若 `tick` 之前无任何 Scroll 事件，返回 `1.0`。
    #[expect(
        clippy::indexing_slicing,
        reason = "idx from saturating_sub on partition_point, always valid"
    )]
    pub fn rate_at(&self, tick: u64) -> f64 {
        let idx = self.segments.partition_point(|&(t, _)| t <= tick);
        if idx == 0 {
            return 1.0;
        }
        self.segments[idx - 1].1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scroll_events(pairs: &[(u64, f64)]) -> Vec<Event<(), bmsrs_chart::NoCustomEvent>> {
        pairs
            .iter()
            .map(|&(tick, rate)| Event::Scroll { tick, rate })
            .collect()
    }

    #[test]
    fn no_scroll_events_returns_default() {
        let events: Vec<Event<(), bmsrs_chart::NoCustomEvent>> = vec![];
        let cache = ScrollCache::build(&events);
        assert!((cache.rate_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.rate_at(480) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn single_scroll_event_affects_subsequent_ticks() {
        let events = make_scroll_events(&[(240, 2.0)]);
        let cache = ScrollCache::build(&events);

        assert!((cache.rate_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.rate_at(239) - 1.0).abs() < 1e-9);
        assert!((cache.rate_at(240) - 2.0).abs() < 1e-9);
        assert!((cache.rate_at(480) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn multiple_scroll_events_take_last() {
        let events = make_scroll_events(&[(0, 1.5), (240, 0.5), (480, 2.0)]);
        let cache = ScrollCache::build(&events);

        assert!((cache.rate_at(0) - 1.5).abs() < 1e-9);
        assert!((cache.rate_at(239) - 1.5).abs() < 1e-9);
        assert!((cache.rate_at(240) - 0.5).abs() < 1e-9);
        assert!((cache.rate_at(479) - 0.5).abs() < 1e-9);
        assert!((cache.rate_at(480) - 2.0).abs() < 1e-9);
        assert!((cache.rate_at(9999) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn matches_linear_scan_implementation() {
        // 验证 cache 结果与原来的 O(n) 扫描一致。
        let events = make_scroll_events(&[(120, 0.8), (360, 1.2), (600, 0.5)]);
        let cache = ScrollCache::build(&events);

        for tick in [0u64, 60, 119, 120, 240, 360, 480, 600, 720, 1000] {
            let expected = linear_scroll_rate_at(&events, tick);
            let actual = cache.rate_at(tick);
            assert!(
                (actual - expected).abs() < 1e-9,
                "mismatch at tick {tick}: expected {expected}, got {actual}"
            );
        }
    }

    /// 模拟原来的 O(n) 线性扫描实现，用于等价性验证。
    fn linear_scroll_rate_at(events: &[Event<(), bmsrs_chart::NoCustomEvent>], tick: u64) -> f64 {
        let end = events.partition_point(|e| e.tick() <= tick);
        let mut rate = 1.0;
        for event in &events[..end] {
            if let Event::Scroll { rate: sc_rate, .. } = event {
                rate = *sc_rate;
            }
        }
        rate
    }
}
