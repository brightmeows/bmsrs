//! osu! 特有的自定义事件与谱面级元数据。

use bmsrs_chart::CustomEvent;

/// osu! 游戏模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OsuMode {
    /// osu!standard。
    #[default]
    Osu,
    /// osu!taiko。
    Taiko,
    /// osu!catch。
    Catch,
    /// osu!mania。
    Mania,
}

/// osu! 谱面级元数据与难度设置。
///
/// 浮点难度值以定点整数保存，避免 `Eq` 与 NaN 的冲突；读取时使用
/// 对应的 getter 还原。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OsuChartMeta {
    /// `.osu` 文件格式版本。
    pub format_version: i32,
    /// 游戏模式。
    pub mode: OsuMode,
    /// 谱面作者。
    pub creator: String,
    /// 原曲来源。
    pub source: String,
    /// 搜索标签。
    pub tags: String,
    /// 原始标题（Unicode）。
    pub title_unicode: String,
    /// 原始艺术家（Unicode）。
    pub artist_unicode: String,
    /// osu! 谱面 ID。
    pub beatmap_id: i32,
    /// osu! 谱面集 ID。
    pub beatmap_set_id: i32,
    /// 音频开始前的静音时长（毫秒）。
    pub audio_lead_in_ms: u32,
    /// 选曲预览时间（毫秒）；`-1` 表示未设置。
    pub preview_time_ms: i32,
    /// HP drain rate，放大 10 倍。
    pub hp_drain_rate_x10: u16,
    /// Overall difficulty，放大 10 倍。
    pub overall_difficulty_x10: u16,
    /// Approach rate，放大 10 倍。
    pub approach_rate_x10: u16,
    /// mania 键数（来自 `CircleSize`）。
    pub key_count: u8,
    /// Slider multiplier，放大 100 倍。
    pub slider_multiplier_x100: u32,
    /// Slider tick rate，放大 100 倍。
    pub slider_tick_rate_x100: u32,
    /// Stack leniency，放大 1000 倍。
    pub stack_leniency_x1000: u16,
}

impl OsuChartMeta {
    /// 返回 HP drain rate。
    #[must_use]
    pub fn hp_drain_rate(&self) -> f32 {
        f32::from(self.hp_drain_rate_x10) / 10.0
    }

    /// 返回 overall difficulty。
    #[must_use]
    pub fn overall_difficulty(&self) -> f32 {
        f32::from(self.overall_difficulty_x10) / 10.0
    }

    /// 返回 approach rate。
    #[must_use]
    pub fn approach_rate(&self) -> f32 {
        f32::from(self.approach_rate_x10) / 10.0
    }

    /// 返回 slider multiplier。
    #[must_use]
    pub fn slider_multiplier(&self) -> f64 {
        f64::from(self.slider_multiplier_x100) / 100.0
    }

    /// 返回 slider tick rate。
    #[must_use]
    pub fn slider_tick_rate(&self) -> f64 {
        f64::from(self.slider_tick_rate_x100) / 100.0
    }

    /// 返回 stack leniency。
    #[must_use]
    pub fn stack_leniency(&self) -> f32 {
        f32::from(self.stack_leniency_x1000) / 1000.0
    }
}

/// osu! 特有的谱面事件。
///
/// tick 由 [`Event`](bmsrs_chart::Event) 统一存储，本枚举的变体不含 tick。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OsuCustomEvent {
    /// 谱面级元数据，在 tick 0 生成一次。
    ChartMeta(OsuChartMeta),
    /// 非继承 timing point 的拍号信息。
    Timing {
        /// 每小节拍数。
        time_signature: u32,
        /// 是否省略首条小节线。
        omit_first_bar_line: bool,
    },
    /// Kiai 开关。
    Kiai(bool),
    /// Break 区间；事件 tick 为开始，`end_tick` 为结束。
    Break {
        /// Break 结束 tick。
        end_tick: u64,
    },
}

impl CustomEvent for OsuCustomEvent {}
