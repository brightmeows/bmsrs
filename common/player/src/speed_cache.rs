//! 预计算的 SPEED 关键帧插值缓存。
//!
//! [`SpeedCache`] 存储已排序的 SPEED 关键帧，在查询时对相邻关键帧
//! 之间的间距值进行线性插值。

use bmsrs_chart::{CustomEvent, Event, EventKind, NoteExt};

/// SPEED 关键帧：脉冲位置与间距倍率。
#[derive(Clone, Copy, Debug)]
struct SpeedKeyframe {
    /// 关键帧的绝对脉冲位置。
    tick: u64,
    /// 此关键帧处的间距倍率。
    rate: f64,
}

/// SPEED 间距插值缓存。
///
/// 构建时从 [`EventKind::Speed`] 事件收集关键帧。`spacing_at(tick)` 在
/// 相邻关键帧之间执行线性插值；若查询位置在首个关键帧之前，返回 `1.0`
/// （默认间距值）。
pub struct SpeedCache {
    /// 按 tick 升序排列的关键帧。
    keyframes: Vec<SpeedKeyframe>,
}

impl SpeedCache {
    /// 从已排序事件列表构建缓存。
    ///
    /// 仅遍历 `Speed` 变体，跳过其他事件类型。
    /// 输入 `events` 必须按 [`Event::tick`] 升序排列（由谱面保证）。
    pub fn build<T: NoteExt, C: CustomEvent>(events: &[Event<T, C>]) -> Self {
        let keyframes: Vec<SpeedKeyframe> = events
            .iter()
            .filter_map(|e| {
                if let EventKind::Speed { rate } = &e.kind {
                    Some(SpeedKeyframe {
                        tick: e.tick(),
                        rate: *rate,
                    })
                } else {
                    None
                }
            })
            .collect();
        Self { keyframes }
    }

    /// 返回 `tick` 处的插值间距倍率。
    ///
    /// 无关键帧或查询在首个关键帧之前 → 返回 `1.0`。
    /// 查询在最后一个关键帧之后 → 返回最后一个关键帧的间距。
    /// 两关键帧之间 → 线性插值。
    pub fn spacing_at(&self, tick: u64) -> f64 {
        // 空关键帧表：默认间距 1.0。
        let Some(first) = self.keyframes.first() else {
            return 1.0;
        };
        if tick < first.tick {
            return 1.0;
        }

        // 找出首个 tick > 查询位置的关键帧。
        let next_idx = self.keyframes.partition_point(|k| k.tick <= tick);
        if next_idx == 0 {
            return 1.0;
        }
        if next_idx >= self.keyframes.len() {
            // keyframes 非空（已由 first() 确认第一个元素存在）。
            #[expect(clippy::unwrap_used, reason = "keyframes confirmed non-empty above")]
            return self.keyframes.last().unwrap().rate;
        }

        // keyframes[next_idx - 1] 与 keyframes[next_idx] 皆存在：
        // next_idx > 0（上方的 == 0 分支），next_idx < len（上方的 >= len 分支）。
        //
        // 直接索引比 .get().unwrap() 更清晰且不需要额外 expect。
        #[expect(clippy::indexing_slicing, reason = "bounds confirmed by checks above")]
        let (prev, next) = (&self.keyframes[next_idx - 1], &self.keyframes[next_idx]);
        // 同一脉冲上的多个 SPEED 关键帧（畸形输入）会导致除零产生 NaN；
        // 此时不插值，直接取后插入关键帧的值（与“后者覆盖前者”语义一致）。
        if next.tick == prev.tick {
            return next.rate;
        }
        #[expect(clippy::cast_precision_loss, reason = "tick offsets fit in f64")]
        let t = (tick - prev.tick) as f64 / (next.tick - prev.tick) as f64;
        (next.rate - prev.rate).mul_add(t, prev.rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_speed_events(pairs: &[(u64, f64)]) -> Vec<Event<(), bmsrs_chart::NoCustomEvent>> {
        pairs
            .iter()
            .map(|&(tick, rate)| Event::speed(tick, rate))
            .collect()
    }

    #[test]
    fn no_events_returns_default() {
        let events: Vec<Event<(), bmsrs_chart::NoCustomEvent>> = vec![];
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at(9999) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn before_first_keyframe_returns_default() {
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at(479) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn after_last_keyframe_returns_last_rate() {
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(480) - 0.5).abs() < 1e-9);
        assert!((cache.spacing_at(9999) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn at_keyframe_exact() {
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(480) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn interpolation_between_keyframes() {
        // 关键帧：tick 0 at 0.5, tick 960 at 1.5
        // tick 480: midpoint, expected 1.0
        let events = make_speed_events(&[(0, 0.5), (960, 1.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(0) - 0.5).abs() < 1e-9);
        assert!((cache.spacing_at(480) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at(960) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn duplicate_tick_speed_events_no_nan() {
        // 畸形输入：两个 SPEED 关键帧位于同一脉冲。插值分母为零，
        // 必须避免 NaN（返回后插入关键帧的值）。
        let events = make_speed_events(&[(480, 0.5), (480, 1.5), (960, 1.0)]);
        let cache = SpeedCache::build(&events);
        let result = cache.spacing_at(720);
        assert!(result.is_finite(), "spacing must be finite, got {result}");
    }

    #[test]
    fn bmspec_6_multiple_speed_interpolation() {
        // 模拟 bmspec-6 多值场景：
        // SPEED01=0.5 at tick 960 (beat 4)
        // SPEED02=1.5 at tick 1440 (beat 6)
        // SPEED03=1.0 at tick 1920 (beat 8)
        let events = make_speed_events(&[(960, 0.5), (1440, 1.5), (1920, 1.0)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(960) - 0.5).abs() < 1e-9, "keyframe 01");
        // keyframe 01→02 插值: tick 1200 位于 960 与 1440 的中点
        // 0.5 + (1.5 - 0.5) * 0.5 = 1.0
        assert!(
            (cache.spacing_at(1200) - 1.0).abs() < 1e-9,
            "interpolation midpoint"
        );
        assert!((cache.spacing_at(1440) - 1.5).abs() < 1e-9, "keyframe 02");
        // beat 7 = tick 1680: 75% between 1440(1.5) and 1920(1.0)
        // 1.5 + (1.0 - 1.5) * (1680-1440)/(1920-1440) = 1.5 + (-0.5 * 0.5) = 1.25
        assert!(
            (cache.spacing_at(1680) - 1.25).abs() < 1e-9,
            "interpolation at beat 7"
        );
        assert!((cache.spacing_at(1920) - 1.0).abs() < 1e-9, "keyframe 03");
    }
}
