//! 用于分支选择的随机数生成器抽象。

use rand::RngExt;

/// 生成 `[1, max]` 范围内随机值的随机数生成器。
///
/// 此 trait 封装了 `#RANDOM N` 的 BMS 规范语义（从 `1..=N` 中均匀选取）。
/// 任意 [`rand::RngExt`] 通过下方的 blanket impl 自动满足此 trait，
/// 因此调用者可直接传入 `StdRng`、`ThreadRng` 或任何其他 `rand` 生成器。
pub trait BranchRng {
    /// 生成 `[1, max]`（含两端）范围内的随机值。
    ///
    /// # Panics
    ///
    /// 当 `max` 为零（空范围）时可能 panic。
    fn gen_range(&mut self, max: u64) -> u64;
}

/// 任意 [`rand::RngExt`] 都是一个 [`BranchRng`]：委托给
/// [`rand::RngExt::random_range`]，作用于闭区间 `1..=max`。
impl<R: RngExt + ?Sized> BranchRng for R {
    fn gen_range(&mut self, max: u64) -> u64 {
        self.random_range(1..=max)
    }
}
