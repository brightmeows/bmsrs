//! 命令行接口定义。

use std::path::PathBuf;

use clap::Subcommand;

/// 谱面内的一个定位点：小节号 + 小节内位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    /// 0 起小节号。
    pub measure: u16,
    /// 小节内位置（分子, 分母）。
    pub pos: (u32, u32),
}

/// `bmsedit` 命令行。
#[derive(Debug, clap::Parser)]
pub struct BmsEdit {
    /// 以 JSON 输出结果（默认人话文本）。
    #[arg(long, global = true)]
    pub json: bool,
    /// `#RANDOM`/`#SWITCH` 分支选择的随机种子。
    #[arg(long, global = true, default_value_t = crate::patch::DEFAULT_SEED)]
    pub seed: u64,
    /// 子命令。
    #[command(subcommand)]
    pub command: Command,
}

/// 子命令集。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// 谱面信息与统计。
    Info {
        /// 谱面文件路径。
        file: PathBuf,
    },
    /// 头部元数据读取与编辑。
    Header {
        /// 谱面文件路径。
        file: PathBuf,
        #[command(subcommand)]
        /// 动作。
        action: HeaderAction,
    },
    /// 音符编辑。
    Notes {
        /// 谱面文件路径。
        file: PathBuf,
        #[command(subcommand)]
        /// 动作。
        action: NotesAction,
    },
    /// 时序（BPM / STOP / SCROLL / 小节长）编辑。
    Timing {
        /// 谱面文件路径。
        file: PathBuf,
        #[command(subcommand)]
        /// 动作。
        action: TimingAction,
    },
    /// 资源定义表（WAV / BMP）操作。
    Defs {
        /// 谱面文件路径。
        file: PathBuf,
        #[command(subcommand)]
        /// 动作。
        action: DefsAction,
    },
    /// 小节结构操作。
    Measure {
        /// 谱面文件路径。
        file: PathBuf,
        #[command(subcommand)]
        /// 动作。
        action: MeasureAction,
    },
    /// 批量变换。
    Transform {
        /// 谱面文件路径。
        file: PathBuf,
        #[command(subcommand)]
        /// 动作。
        action: TransformAction,
    },
    /// 校验谱面结构不变量。
    Validate {
        /// 谱面文件路径。
        file: PathBuf,
    },
    /// 常见谱面问题检查。
    Lint {
        /// 谱面文件路径。
        file: PathBuf,
    },
}

/// 头部子命令。
#[derive(Debug, Subcommand)]
pub enum HeaderAction {
    /// 读取头部：指定 `key` 读取单项，省略读取全部。
    Get {
        /// 头部命令名（如 `TITLE`、`BPM`、`WAV01`）。
        key: Option<String>,
    },
    /// 设置头部值（已存在则原位替换，否则插入）。
    Set {
        /// `KEY=VALUE` 对，可多个（如 `TITLE=曲名 BPM=180`）。
        #[arg(required = true, value_name = "KEY=VALUE")]
        pairs: Vec<String>,
    },
}

/// 音符子命令。
#[derive(Debug, Subcommand)]
pub enum NotesAction {
    /// 列出音符（可按小节 / lane / 类型过滤）。
    List {
        /// 起始小节（含）。
        #[arg(long)]
        from_measure: Option<u16>,
        /// 结束小节（含）。
        #[arg(long)]
        to_measure: Option<u16>,
        /// 按逻辑 lane 过滤（整数 = 按键，`sc` = 转盘）。
        #[arg(long)]
        lane: Option<String>,
    },
    /// 添加一个音符。
    Add {
        /// 小节号（0 起）。
        #[arg(long)]
        measure: u16,
        /// 逻辑 lane（整数 = 按键，`sc` = 转盘）。
        #[arg(long)]
        lane: String,
        /// 小节内位置，分数（`1/4`）或小数（`0.25`）。
        #[arg(long)]
        pos: String,
        /// 音频定义 ID（wav 索引，如 `01`）；bmson 为音频文件名。
        #[arg(long)]
        def: String,
        /// 放置为 BGM（不可玩）音符。
        #[arg(long)]
        bgm: bool,
        /// 长音终点：另一位置的 `measure:pos`。
        #[arg(long)]
        ln_to: Option<String>,
    },
    /// 删除音符。
    Remove {
        /// 小节号。
        #[arg(long)]
        measure: u16,
        /// 逻辑 lane。
        #[arg(long)]
        lane: String,
        /// 小节内位置；省略时删除该 lane 在此小节的全部音符。
        #[arg(long)]
        pos: Option<String>,
    },
    /// 移动音符。
    Move {
        /// 逻辑 lane。
        #[arg(long)]
        lane: String,
        /// 源位置 `MEASURE:POS`。
        from: String,
        /// 目标位置 `MEASURE:POS`。
        to: String,
    },
    /// 复制小节范围内的全部音符到目标小节。
    Copy {
        /// 起始小节（含）。
        #[arg(long)]
        from_measure: u16,
        /// 结束小节（含）。
        #[arg(long)]
        to_measure: u16,
        /// 目标小节号（源范围平移到此）。
        #[arg(long)]
        dest_measure: u16,
    },
}

/// 时序子命令。
#[derive(Debug, Subcommand)]
pub enum TimingAction {
    /// 列出全部时序事件（BPM / STOP / SCROLL / 小节长）。
    List {},
    /// 设置初始 BPM。
    Bpm {
        /// BPM 值。
        bpm: f64,
    },
    /// 设置或移除某小节的小节长倍率（1.0 = 4/4）。
    Length {
        /// 小节号。
        measure: u16,
        /// 倍率（如 `3/4`、`1.5`）；省略时移除定义。
        #[arg(allow_hyphen_values = true, value_name = "RATIO")]
        ratio: Option<String>,
    },
    /// 在指定位置插入 STOP。
    StopAdd {
        /// 位置 `MEASURE:POS`。
        at: String,
        /// 停止时长（1/192 小节单位）。
        duration: u32,
    },
    /// 移除指定位置的 STOP。
    StopRemove {
        /// 位置 `MEASURE:POS`。
        at: String,
    },
    /// 在指定位置插入 SCROLL 倍率变更。
    ScrollAdd {
        /// 位置 `MEASURE:POS`。
        at: String,
        /// 滚动倍率（支持负值反向滚动）。
        #[arg(allow_hyphen_values = true)]
        ratio: f64,
    },
    /// 移除指定位置的 SCROLL 倍率变更。
    ScrollRemove {
        /// 位置 `MEASURE:POS`。
        at: String,
    },
}

/// 定义表子命令。
#[derive(Debug, Subcommand)]
pub enum DefsAction {
    /// 列出定义与引用计数。
    List {
        /// 定义类别（`wav` 或 `bmp`，默认 `wav`）。
        #[arg(default_value = "wav")]
        kind: String,
    },
    /// 重命名定义（全部引用同步）。
    Rename {
        /// 定义类别（`wav` / `bmp`）。
        #[arg(default_value = "wav")]
        kind: String,
        /// 旧 ID。
        from: String,
        /// 新 ID。
        to: String,
    },
    /// 去重：相同文件的多个定义合并到最小 ID。
    Dedup {
        /// 定义类别（`wav` / `bmp`）。
        #[arg(default_value = "wav")]
        kind: String,
    },
    /// 空档压缩：重编号消除 ID 空洞。
    Compact {
        /// 定义类别（`wav` / `bmp`）。
        #[arg(default_value = "wav")]
        kind: String,
    },
}

/// 小节子命令。
#[derive(Debug, Subcommand)]
pub enum MeasureAction {
    /// 在指定小节前插入空小节。
    Insert {
        /// 插入位置（该小节及其后整体后移）。
        at: u16,
        /// 插入数量。
        #[arg(long, default_value_t = 1)]
        count: u16,
    },
    /// 删除小节范围。
    Remove {
        /// 起始小节（含）。
        at: u16,
        /// 删除数量。
        #[arg(long, default_value_t = 1)]
        count: u16,
    },
    /// 复制小节内容到目标小节（覆盖语义按通道组合并）。
    CloneContent {
        /// 源小节。
        from: u16,
        /// 目标小节。
        to: u16,
    },
}

/// 变换子命令。
#[derive(Debug, Subcommand)]
pub enum TransformAction {
    /// 谱面镜像（左右 lane 对调，转盘不动）。
    Mirror {},
    /// 按 grid 稀疏化：保留 grid 网格上的音符，其余删除。
    Sparsify {
        /// 网格分母（如 4 = 保留 1/4 位置上的音符）。
        grid: u32,
    },
}

/// 解析 `MEASURE:POS` 位置串；`POS` 省略为小节头。
///
/// `POS` 支持 `1/4` 分数与小数 `0.25` 两种写法。
///
/// # Errors
///
/// 位置格式非法时返回错误消息。
pub fn parse_spot(s: &str) -> Result<Spot, String> {
    let (measure_part, pos_part) = s.split_once(':').unwrap_or((s, "0"));
    let measure: u16 = measure_part
        .parse()
        .map_err(|_err| format!("invalid measure `{measure_part}`"))?;
    let pos = parse_pos(pos_part)?;
    Ok(Spot { measure, pos })
}

/// 解析小节内位置为分数。
///
/// # Errors
///
/// 位置格式非法、越界或无法精确到已知网格时返回错误消息。
pub fn parse_pos(s: &str) -> Result<(u32, u32), String> {
    if let Some((numer_raw, denom_raw)) = s.split_once('/') {
        let numer: u32 = numer_raw
            .parse()
            .map_err(|_err| format!("invalid numerator `{numer_raw}`"))?;
        let denom: u32 = denom_raw
            .parse()
            .map_err(|_err| format!("invalid denominator `{denom_raw}`"))?;
        if denom == 0 {
            return Err("denominator must not be zero".to_owned());
        }
        return Ok((numer, denom));
    }
    let value: f64 = s
        .parse()
        .map_err(|_err| format!("invalid position `{s}`"))?;
    if !(0.0..=1.0).contains(&value) {
        return Err(format!("position `{s}` outside [0, 1]"));
    }
    // 小数化为分母 ≤ 1024 的最简分数。
    for denom in [
        1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 1024,
    ] {
        let scaled = value * f64::from(denom);
        let rounded = scaled.round();
        if (scaled - rounded).abs() < 1e-9 && rounded >= 0.0 && rounded <= f64::from(u32::MAX) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "rounded is checked to be a non-negative integer within u32 range"
            )]
            return Ok((rounded as u32, denom));
        }
    }
    Err(format!("position `{s}` has no exact grid within 1/1024"))
}

/// 解析逻辑 lane 输入（整数 = 按键，`sc` = 转盘）。
///
/// # Errors
///
/// 输入既非整数也非 `sc`/`scratch` 时返回错误消息。
pub fn parse_lane(s: &str) -> Result<crate::lane_map::LaneRef, String> {
    match s.to_ascii_lowercase().as_str() {
        "sc" | "scratch" => Ok(crate::lane_map::LaneRef::Scratch(1)),
        other => {
            let n: u8 = other
                .parse()
                .map_err(|_err| format!("invalid lane `{s}` (integer key or `sc`)"))?;
            if n == 0 {
                return Err("lane numbering starts at 1".to_owned());
            }
            Ok(crate::lane_map::LaneRef::Key(n))
        }
    }
}
