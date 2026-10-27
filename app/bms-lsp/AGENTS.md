# bms-lsp

## 定位

BMS 语言服务器：通用 stdio，先服务 Pi/Neovim。依赖 `bms-analysis` 的
诊断与符号索引，只做 LSP 协议适配。`publish = false`。

## 功能

| LSP 能力 | 数据来源 |
|----------|----------|
| 诊断 | `Analysis::diagnostics()` |
| hover | `header_doc()` / `channel_doc()` / 符号索引 |
| 补全 | 头部命令表、通道表、定义 ID |
| 跳转定义/引用 | `SymbolIndex` |
| 文档符号 | CST 行节点 |
| semantic tokens | CST token 种类 |
| 折叠 | 控制流块节点 |

## 架构

- 同步 `lsp-server` 主循环，rowan 红树留在主线程。
- 全量同步（`TextDocumentSyncKind::FULL`）；每次 didChange 重新分析。
- `LineIndex` 负责 UTF-8 字节偏移 ↔ LSP UTF-16 位置。
- 文本以客户端发送的 Unicode 为准；编码检测在 `bms-source`，不进入 LSP 热路径。

## 测试

```bash
cargo test -p bms-lsp
cargo build -p bms-lsp
```

`LineIndex` 单测覆盖 ASCII、第二行与日文 UTF-16 计数。协议冒烟测试可用
stdio 发送 `initialize` / `didOpen` / `hover` / `definition` / `shutdown`。

## Always / Ask / Never

### Always

- 新增 LSP 能力时同步 `capabilities()` 与请求分发
- 诊断/位置都经 `LineIndex` 转换，不直接混用字节与 UTF-16
- 文档变更后重新分析并发布诊断

### Ask

- 改成增量同步或增量解析
- 引入 async runtime / `tower-lsp-server`

### Never

- 在 LSP 层做 BMS 语义解析（归 `bms-analysis` / `bms-parser`）
- 把 rowan 红树跨线程共享
- 在没有客户端文本时自行猜测文件内容
