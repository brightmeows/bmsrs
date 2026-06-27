# bms-tokenizer-derive

## 定位

`#[derive(BmsTokenAttr)]` proc-macro crate。自动处理所有 `#[bms_token("...")]` 模式。

纯编译时逻辑，无运行时依赖。

## 三种模式

| 模式 | 检测条件 | 生成内容 |
|------|----------|----------|
| **Command** | `#[bms_token("#BPM{id} {value}")]` 或 `#[bms_token("#TITLE {}")]` | `try_match_header` + `format_header` |
| **Literal** | `#[bms_token("0")]`（无 `#`/`%` 前缀）| `FromStr` + `Display` |
| **Dispatch** | 无 `#[bms_token]`，全部是单 tuple 变体 | `BmsHeader::try_match_header` |

## 模块结构

| 文件 | 职责 |
|------|------|
| `lib.rs` | derive macro 入口，模式检测 |
| `parse.rs` | template 字符串解析（`"#BPM{id} {value}"` → `BmsTokenTemplate`）|
| `codegen.rs` | 生成 `try_match_header` / `format_header`（command 模式 + dispatch 模式）|
| `value_codegen.rs` | 生成 `FromStr` / `Display`（literal 模式）|

## 生成的函数

| 函数 | 模式 | 作用 |
|------|------|------|
| `try_match_header` | Command | 子 enum 级解析入口 |
| `format_header` | Command | enum 变体 → `(command, value)` |
| `BmsHeader::try_match_header` | Dispatch | 顶层调度 |
| `FromStr` / `Display` | Literal | 值 ↔ 字符串映射 |

## 属性

| 属性 | 效果 |
|------|------|
| `#[bms_token("...")]` | command pattern 或 literal value |
| `#[bms_fallback]` | Command 模式：失败 → `Ok(None)`；Dispatch 模式：跳过此变体 |
| `#[doc(hidden)]` | 跳过 codegen（用于 phantom 变体）|

## `C` 字段检测

enum 的第一个类型参数 = 字符串容器。匹配该 ident 的字段通过 `<C as From<&str>>::from(value)` 解析，而非 `FromStr`/`BmsValue`。

## 错误映射

macro 将每个 `FromStr::Err` 通过 `IntoTokensError` 转换为 `BmsTokenizeError`。

| 内置实现 | 说明 |
|----------|------|
| `ParseIntError` / `ParseFloatError` | 标准库整数/浮点解析错误 |
| `ParseBmsValueError` | BMS 通用值解析错误 |
| `BmsChannelIdError` | 通道号解析错误 |
| `ParseDifficultyError` | 难度等级解析错误 |

自定义 `FromStr` 类型实现 `IntoTokensError` 以控制错误变体。

## 测试

```bash
# 通过 bms-tokenizer 的 roundtrip 测试间接测试
cargo test -p bms-tokenizer --test bms_token
# 多态测试
cargo test -p bms-tokenizer --test generic
```

proc-macro 单元测试在 `parse.rs` 中（使用 `proc_macro2` 类型）。
