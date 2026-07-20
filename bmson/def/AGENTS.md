# bmson-def

## 定位

bmson JSON 格式类型定义（v0/v1/v2）。

## 模块结构

| 路径 | 内容 |
|------|------|
| `common.rs` | 通用类型（`NoteEvent`、`BpmEvent`、`BGA`、`ModeHint` 等）——从 crate 根 re-export |
| `v0.rs` / `v1.rs` | 版本特有类型 + `From`/`TryFrom` 转换 trait |
| `lib.rs`（根） | v2 版本的特有类型（`Bmson` v2、`SongInfo`、`ChartInfo`、`ChartData`）——在 crate 根定义，无需独立 mod |

`v0` 和 `v1` 是**独立模块**——各有自己的 `Bmson`、`BmsonInfo`、`SoundChannel`。
从 `bmson_def::v0` 或 `bmson_def::v1` 导入。

## 版本检测

`DetectedVersion::detect(json)` → `DetectedVersion`（V0/V1/V2）——扫描 JSON `"version"` 字段。
轻量实现，无需 `serde_json`。

## 依赖

`serde_json` 和 `chumsky` 是 **dev-only** 依赖——lib 本身不依赖任何 JSON 库。
测试直接用 `serde_json::from_str`/`to_string` 与版本特有类型交互。
`chumsky` 用于示例 `bmson_parser`（支持错误恢复的 JSON 解析器演示）。

## 示例

```bash
cargo run --example bmson_parser -p bmson-def
```

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| v2 的独立 mod 结构 | 与 v0/v1 不同，v2 是目录而非单个文件 |
| 版本间转换通过 `From` | `Bmson::from(v0_bmson)` 统一升版到 v2 schema |
