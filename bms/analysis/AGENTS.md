# bms-analysis

## 定位

BMS 分析层：在 `bms-syntax` 的 CST 上产出编辑器数据。不依赖 LSP 类型，
协议适配由独立仓库 `bms-lsp` 负责。

## 产出

| 数据 | 说明 |
|------|------|
| `Analysis::diagnostics()` | CST 语法/控制流诊断、tokenizer 错误、重复定义、缺失引用 |
| `Analysis::index()` | WAV / BMP / BGA / BPM / STOP / SCROLL / SPEED / EXRANK / CHANGEOPTION / TEXT / SEEK 的定义与引用 |
| `header_doc()` | 头部命令中文说明，按命令前缀匹配（`WAV01` → `WAV`） |
| `channel_doc()` | 通道中文说明，支持音符通道分组 |

## 索引规则

- 定义：`#WAVxx` / `#EXWAVxx`、`#BMPxx` / `#EXBMPxx`、`#BGAxx` / `#@BGAxx` /
  `#SWBGAxx` / `#ARGBxx`、`#BPMxx` / `#EXBPMxx`、`#STOPxx`、`#SCROLLxx`、
  `#SPEEDxx`、`#EXRANKxx`、`#CHANGEOPTIONxx`、`#SEEKxx`、`#TEXTxx` / `#SONGxx`
- 引用：通道体的非 `00` 两字符值；通道种类决定资源种类
- 非规范地址（如 `#1:0101`）在 Header 行上兜底尝试 `parse_message_line`，仍建引用索引
- BGA 通道（04/06/07/0A）允许回退到 `#BMPxx` 定义
- ID 统一转大写做大小写不敏感匹配

## 诊断规则

- CST 语法错误 → Error；控制流结构错误 → Warning
- tokenizer 值解析错误 → Error
- 同种类同 ID 多定义 → 每个定义一条重复警告
- 引用找不到定义 → 引用处一条缺失警告

## 测试

```bash
cargo test -p bms-analysis
```

覆盖：定义/引用索引、缺失引用、重复定义、BGA→BMP 回退、控制流诊断、
头部值错误、文档查表。

## Always / Ask / Never

### Always

- 新增资源种类时同步 `SymbolKind`、定义前缀表、通道映射与诊断
- 自定义 header 前缀走 `Analysis::from_text_with(&ParseOptions)`
- 定义与引用都保留 `TextRange`，供 LSP 跳转/高亮
- 文档查表用最长前缀匹配，避免 `BPM` 抢 `EXBPM`

### Ask

- 修改“什么算定义/引用”的边界（会影响诊断噪声）
- 引入需要读取文件系统或 LSP 类型的依赖

### Never

- 在本 crate 依赖 LSP 协议类型
- 把分支选择结果当成唯一真相；索引必须覆盖所有分支
- 对同一范围重复堆同一条诊断
