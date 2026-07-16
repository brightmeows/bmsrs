//! BMS 模式族布局：使用 [`BmsLayout`] 进行通道到轨道的映射。
//!
//! 每个族表示为一个零大小 newtype，将解码后的 [`BmsChannel`]
//! `(player, lane)` 对映射到 `(NoteSide, Lane)` 位置。
//!
//! 族**不**携带按键数：一张谱面是 5K 还是 7K 取决于其音符实际使用的
//! 轨道，而非族本身。

use std::num::NonZeroU8;

use bmsrs_chart::mode::{Lane, NoteSide};

/// 解码后的 BMS 通道标识符：从原始 BMS 通道字节中提取的 `(player, lane)`
/// 对（例如 `"19"` → `{ player: 1, lane: 9 }`）。
///
/// 构造时校验 `player ∈ {1, 2}` 与 `lane ∈ {1..=9}`，因此下游代码可以
/// 安全地调用 [`note_side`](Self::note_side) 与 [`lane`](Self::lane) 而
/// 无需额外检查。轨道值为解码后的数字（1–9），**而非**原始通道字节。
/// 已校验的 BMS `(player, lane)` 通道对。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BmsChannel {
    /// 玩家编号（1 或 2）。
    player: u8,
    /// 轨道编号（1–9）。
    lane: u8,
}

impl BmsChannel {
    /// 从解码后的 `(player, lane)` 创建新的 `BmsChannel`。
    ///
    /// 若 `player` 不为 1 或 2，或 `lane` 不在 1–9 范围内，返回 `None`。
    #[must_use]
    pub const fn new(player: u8, lane: u8) -> Option<Self> {
        if (player == 1 || player == 2) && lane >= 1 && lane <= 9 {
            Some(Self { player, lane })
        } else {
            None
        }
    }

    /// 玩家侧（1 或 2）。
    #[must_use]
    pub const fn player(self) -> u8 {
        self.player
    }

    /// 轨道编号（1–9）。
    #[must_use]
    pub const fn lane(self) -> u8 {
        self.lane
    }

    /// 以 [`NoteSide`] 返回玩家侧。
    #[must_use]
    pub const fn note_side(self) -> NoteSide {
        match self.player {
            1 => NoteSide::P1,
            _ => NoteSide::P2,
        }
    }
}

/// BMS 侧映射：将 [`BmsChannel`] 解码为 `(NoteSide, Lane)` 位置对，若
/// 通道应被丢弃则返回 `None`。
///
/// 音符种类（普通 / 长音 / 地雷 / 不可见）由处理器在此映射之后设置。
pub trait BmsLayout {
    /// 将解码后的 BMS 通道映射到音符的位置。
    #[must_use]
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)>;
}

/// 从调用处已知 ≥ 1 的值构造 `NonZeroU8`。
const fn nz(n: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(n)
}

/// BME 族：覆盖 `beat-5k`、`beat-7k`、`beat-10k`、`beat-14k`（及 `dj-*`
/// 别名）。一张映射表服务全部；特定谱面的按键数由其音符实际使用情况
/// 决定。
///
/// 物理布局（左 → 右）：每侧 `KEY1-5 | SC | KEY6-7`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bme;

impl BmsLayout for Bme {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Scratch(nz(1)?),
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// Nanasi 族：BME 按键加上通道 `17`/`27` 上的脚踏板。覆盖 nanasi 与
/// Angolmois 踏板变体（它们的通道映射完全相同；SC 渲染侧差异属于渲染器
/// 关注范围，而非映射）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Nanasi;

impl BmsLayout for Nanasi {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Scratch(nz(1)?),
            7 => Lane::FootPedal,
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// 原生 PMS 族：单人 9 按键布局，其 KEY6-9 复用 2P 通道 `22-25`。覆盖
/// `popn-9k`、`popn-5k`（子集）及 `pomu-battle` 3K 子集。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pms;

impl BmsLayout for Pms {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let key = match (ch.note_side().as_u8(), ch.lane()) {
            (1, 1) => Lane::Key(nz(1)?),
            (1, 2) => Lane::Key(nz(2)?),
            (1, 3) => Lane::Key(nz(3)?),
            (1, 4) => Lane::Key(nz(4)?),
            (1, 5) => Lane::Key(nz(5)?),
            (2, 2) => Lane::Key(nz(6)?),
            (2, 3) => Lane::Key(nz(7)?),
            (2, 4) => Lane::Key(nz(8)?),
            (2, 5) => Lane::Key(nz(9)?),
            _ => return None,
        };
        Some((NoteSide::P1, key))
    }
}

/// PMS BME 型族：每侧 9 按键，采用 BME 通道形状
/// （`KEY6=18 KEY7=19 KEY8=16 KEY9=17`）。注意：BME 族读取为 Scratch /
/// 空闲的通道 `16`/`17`，在此承载 `KEY8`/`KEY9` —— 这正是需要独立族的原因。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PmsBme;

impl BmsLayout for PmsBme {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Key(nz(8)?),
            7 => Lane::Key(nz(9)?),
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// PMS 布局变体：Standard（原生 PMS）或 BME-type（PMS BME 型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmsLayout {
    /// 原生 PMS 布局：2P 通道 `22-25` 上的 KEY6-9。
    Standard,
    /// PMS BME 型布局：使用 BME 通道形状（`KEY6=18 KEY7=19 KEY8=16 KEY9=17`）。
    BmeType,
}

impl PmsLayout {
    /// 从已解析的 BMS 数据自动检测 PMS 变体。
    ///
    /// 扫描 note、long note 与 mine 事件中的 `(player, lane)` 对：
    /// - `player == 2 && lane in 2..=5` 指示 Standard（原生 PMS）。
    /// - `player == 1 && lane in 6..=9` 指示 BME-type。
    /// - 若同时出现两种指示，Standard 优先（safe default）。
    /// - 若无可指示的事件，返回 Standard。
    #[must_use]
    pub fn detect(bms: &bms_parser::Bms) -> Self {
        let mut has_standard = false;
        let mut has_bme = false;

        for ev in &bms.messages.note_events {
            check_pms_channel(ev.player, ev.lane, &mut has_standard, &mut has_bme);
        }
        for ev in &bms.messages.long_note_events {
            check_pms_channel(ev.player, ev.lane, &mut has_standard, &mut has_bme);
        }
        for ev in &bms.messages.mine_events {
            check_pms_channel(ev.player, ev.lane, &mut has_standard, &mut has_bme);
        }

        if has_standard {
            Self::Standard
        } else if has_bme {
            Self::BmeType
        } else {
            Self::Standard
        }
    }
}

/// 检查单个 `(player, lane)` 对是否指示 PMS Standard 或 BME-type 变体。
fn check_pms_channel(player: u8, lane: u8, std: &mut bool, bme: &mut bool) {
    if player == 2 && (2..=5).contains(&lane) {
        *std = true;
    }
    if player == 1 && (6..=9).contains(&lane) {
        *bme = true;
    }
}

/// DSC/FPP + OCT/FP 族：双侧布局，最多两个 Scratch 与一个脚踏板。
/// DSC/FPP（双 Scratch，无踏板）与 OCT/FP（13 按键 + 第二 Scratch + 踏板）
/// 均为该族按键集的子集。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DscOctFp;

impl BmsLayout for DscOctFp {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        Some(match (ch.note_side().as_u8(), ch.lane()) {
            (1, 1) => (NoteSide::P1, Lane::Key(nz(1)?)),
            (1, 2) => (NoteSide::P1, Lane::Key(nz(2)?)),
            (1, 3) => (NoteSide::P1, Lane::Key(nz(3)?)),
            (1, 4) => (NoteSide::P1, Lane::Key(nz(4)?)),
            (1, 5) => (NoteSide::P1, Lane::Key(nz(5)?)),
            (1, 6) => (NoteSide::P1, Lane::Scratch(nz(1)?)),
            (1, 8) => (NoteSide::P1, Lane::Key(nz(6)?)),
            (1, 9) => (NoteSide::P1, Lane::Key(nz(7)?)),
            (2, 1) => (NoteSide::P2, Lane::FootPedal),
            (2, 2) => (NoteSide::P2, Lane::Key(nz(1)?)),
            (2, 3) => (NoteSide::P2, Lane::Key(nz(2)?)),
            (2, 4) => (NoteSide::P2, Lane::Key(nz(3)?)),
            (2, 5) => (NoteSide::P2, Lane::Key(nz(4)?)),
            (2, 6) => (NoteSide::P2, Lane::Scratch(nz(2)?)),
            (2, 8) => (NoteSide::P2, Lane::Key(nz(5)?)),
            (2, 9) => (NoteSide::P2, Lane::Key(nz(6)?)),
            _ => return None,
        })
    }
}
