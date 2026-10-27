//! `#RANDOM` / `#SWITCH` 控制流命令。
//!
//! 这些命令允许单个 BMS 文件包含多个谱面变体。
//! 分词器原样保留所有分支；选择保留哪个分支是
//! 后续管道阶段（解析器/处理器）的职责。

use crate::BmsTokenAttr;

/// 用于随机谱面分支的控制流头部。
///
/// BMS 支持两种分支构造：
///
/// - **`#RANDOM` 块**：`#RANDOM N` → `#IF k` … `#ENDIF` × N → `#ENDRANDOM`。
///   解析时，在 `[1, N]` 中选择一个整数；仅保留匹配的
///   `#IF` 分支。
/// - **`#SWITCH` 块**：`#SWITCH N` → `#CASE k` … `#DEF` … `#ENDSW`。
///   类似于 `#RANDOM`，但 `#CASE` 匹配一个整数值，而 `#DEF`
///   提供默认回退。
///
/// `#SETRANDOM` / `#SETSWITCH` 强制指定分支（用于工具与
/// 测试）。`#RONDAM` 是被接受的 `#RANDOM` 拼写错误的常见别名。
///
/// 嵌套与引擎兼容性较为复杂——完整细节见 BMS 命令备忘录。
#[derive(Debug, Clone, PartialEq, Eq, BmsTokenAttr)]
pub enum BmsHeaderControlFlow {
    /// `#RANDOM N`（或 `#RONDAM`）——开始一个随机分支块。
    ///
    /// `N` 是分支数；引擎在 `[1, N]` 中取一个值。
    /// `#RONDAM` 是某些播放器识别的历史拼写错误。
    #[bms_token("#RANDOM {}")]
    #[bms_token("#RONDAM {}")]
    Random(u64),
    /// `#SETRANDOM N`——强制指定随机值而非随机滚动。
    ///
    /// 供工具（如预览、IR 回放）用于确定性地选择分支。
    #[bms_token("#SETRANDOM {}")]
    SetRandom(u64),
    /// `#ENDRANDOM`——关闭当前 `#RANDOM` 块。
    ///
    /// 在嵌套 `#RANDOM` 块时推荐使用，因为某些播放器
    /// （nanasi）需要它才能正确工作。
    #[bms_token("#ENDRANDOM")]
    EndRandom,
    /// `#IF N`——当随机值等于 `N` 时激活的分支。
    #[bms_token("#IF {}")]
    If(u64),
    /// `#ELSEIF N`——替代分支（类似 `else if`）。
    #[bms_token("#ELSEIF {}")]
    ElseIf(u64),
    /// `#ELSE`——当无 `#IF` / `#ELSEIF` 匹配时的默认分支。
    #[bms_token("#ELSE")]
    Else,
    /// `#ENDIF`——关闭当前 `#IF` / `#ELSEIF` / `#ELSE` 链。
    ///
    /// 来自各引擎的常见拼写错误：
    /// - `#END IF`（IIDXv/HDX，误解的空格）
    /// - `#END`（Angolmois/Sonorous，部分匹配）
    /// - `#IFEND`（另一种顺序）
    #[bms_token("#ENDIF")]
    #[bms_token("#END IF")]
    #[bms_token("#END")]
    #[bms_token("#IFEND")]
    EndIf,
    /// `#SWITCH N`——开始一个含 `N` 个 case 的 switch 块。
    ///
    /// 引擎在 `[1, N]` 中取一个值；当值等于 `k` 时 `#CASE k` 激活。
    #[bms_token("#SWITCH {}")]
    Switch(u64),
    /// `#SETSWITCH N`——强制指定 switch 值（类似
    /// `#SETRANDOM`）。
    #[bms_token("#SETSWITCH {}")]
    SetSwitch(u64),
    /// `#ENDSW` / `#ENDSWITCH`——关闭当前 `#SWITCH` 块。
    #[bms_token("#ENDSW")]
    #[bms_token("#ENDSWITCH")]
    EndSwitch,
    /// `#CASE N`——当 switch 值等于 `N` 时激活的分支。
    #[bms_token("#CASE {}")]
    Case(u64),
    /// `#SKIP`——`#SWITCH` 块内的 fall-through 跳转标记（无参数）。
    ///
    /// 当标签匹配的 `#CASE` 与下一个 `#CASE`/`#DEF` 之间存在 `#SKIP` 时，
    /// 解析从 `#SKIP` 跳到 `#ENDSW`，不再执行后续分支。
    #[bms_token("#SKIP")]
    Skip,
    /// `#DEF`——`#SWITCH` 块内的默认分支。
    #[bms_token("#DEF")]
    Def,
}
