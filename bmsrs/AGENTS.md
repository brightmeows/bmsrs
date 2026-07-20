# bmsrs

## 定位

公共 re-export facade。无新逻辑。

## 模块结构

`pub mod` 树镜像工作区目录结构：

```text
bms::{tokenizer, control_flow, parser, processor}
bmson::{def, processor}
chart
player
```

## 职责

| 操作 | 时机 |
|------|------|
| 新增 workspace crate | 在 `lib.rs` 加 `pub mod` + `pub use new_crate::*;` |
| 占位 crate | 无公开 API 的不 re-export |

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| re-export 是保底的 | 每个 crate 仍是独立依赖，可直接 `Cargo.toml` 引用 |
| 不重导出依赖 | 只 re-export workspace 内 crate |
