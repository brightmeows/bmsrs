# bmsrs

BMS / BMSON 谱面格式的 Rust 解析、转换与模拟播放工具包。

> 预发布状态：`0.1.0` 尚未发布，公开接口可能变动。

## 简介

bmsrs 处理两种谱面格式：

- BMS：文本格式，提供从原始字节到结构化模型的完整管道。
- BMSON：JSON 格式，支持 v0、v1、v2 类型定义与升版。

两条管道输出统一的 `Chart<T>` 中央 IR。渲染器、播放器与工具只需消费这一层。

项目使用 Rust 2024 edition，MSRV 为 1.95。

## 管道

| 输入 | 处理 | 输出 |
|------|------|------|
| BMS 字节 | `bms-source`、`bms-syntax`、`bms-parser` | `Bms` |
| `Bms` | `bms-processor` | `Chart` |
| BMSON JSON | `bmson-def`、`bmson-processor` | `Chart` |
| `Chart` | `bmsrs-player` | 时间轴查询 |

`bms-analysis` 直接消费 CST，提供诊断、符号索引与编辑器数据。
`bms-writer` 把 `Bms` 规范化重写为 BMS 文本。

## Crate 一览

| Crate | 职责 |
|-------|------|
| `bms-source` | BMS 源文本层：字节、编码、行索引、字节保真编辑 |
| `bms-syntax` | BMS 语法层：rowan CST、词法、控制流查询 |
| `bms-values` | BMS 单行语法与类型化值 |
| `bms-parser` | BMS 语义层：CST 转换为 `Bms` |
| `bms-processor` | `Bms` 转换为 `Chart` |
| `bms-writer` | `Bms` 规范化重写为 BMS 文本 |
| `bms-analysis` | 诊断、符号索引与编辑器数据 |
| `bmson-def` | BMSON v0、v1、v2 类型定义 |
| `bmson-processor` | `Bmson` 转换为 `Chart` |
| `bmsrs-chart` | 格式无关的 `Chart<T>` 中央 IR |
| `bmsrs-player` | `Chart` 的纯仿真层 |
| `bmsrs` | 公共 re-export facade |

## 快速开始

`0.1.0` 尚未发布。当前请使用 git 依赖：

```toml
[dependencies]
bmsrs = { git = "https://github.com/brightmeows/bmsrs" }
```

### BMS 到播放器

```rust
use std::time::Duration;

use bmsrs::bms::processor::BmsProcessor;
use bmsrs::bms::{ErrorStrategy, from_bytes};
use bmsrs::player::Player;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("chart.bms")?;
    let (bms, _warnings) = from_bytes(bytes, |_| 1, ErrorStrategy::CollectAll)?;
    let processed = BmsProcessor::process_default(&bms)?;

    let mut player = Player::new(processed.chart)?;
    player.advance(Duration::from_secs(1));

    println!("current tick: {}", player.current_tick());
    Ok(())
}
```

`from_bytes` 的第二个参数是 `#RANDOM` 与 `#SWITCH` 的分支选择函数。示例固定选择第一个分支；已经是 UTF-8 文本时可改用 `from_text`。

### BMSON 到 Chart

```rust
use bmsrs::bmson::def::Bmson;
use bmsrs::bmson::processor::BmsonProcessor;

fn load(json: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bmson: Bmson<'_> = serde_json::from_str(json)?;
    let processed = BmsonProcessor::process_default(&bmson)?;

    println!("events: {}", processed.chart.data.events.len());
    Ok(())
}
```

需要在项目中自行添加 `serde_json`。v0 与 v1 文件先通过 `Bmson::from` 升版到 v2。

## 开发

常用检查：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --quiet
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

提交前运行全部 hook：

```bash
pre-commit run --all-files
```

播放器热路径基准：

```bash
cargo bench -p bmsrs-player
```

CI 还会运行 `cargo deny check` 与 MSRV 1.95 检查。

各 crate 的职责与开发约定记录在 `AGENTS.md`。子目录规则见 `bms/AGENTS.md`、`bmson/AGENTS.md` 与 `common/AGENTS.md`。

## 许可证

Apache-2.0，见 [`LICENSE`](LICENSE)。
