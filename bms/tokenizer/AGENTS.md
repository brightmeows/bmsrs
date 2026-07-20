# bms-tokenizer

## 定位

BMS 语法分析第一关：原始文本 → 结构化 token 流。

## 关键设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 通道分类时机 | tokenizer 阶段 | 下游 parser 无需重复解析通道号 |
| 跨命令引用保留 | 不解析跨命令引用，保留 raw index | 引用语义（WAV→定义、`#BGA`→`#BMP`）归 processor；`#BASE` 模式由 parser 决定 |
| 字符集 | Base62（最宽松） | 统一入口，`#BASE` 声明在 parser 层生效 |
| 注释预处理 | 独立 `preprocess()` 函数 | 保持 tokenizer 零拷贝路径不受影响 |

## 目标

每个 token 的存在只为三个目的：
1. **Roundtrip 保真** — `format_header` 还原原始行
2. **单行验证** — 语法 + 单行值解释（`Rank` 枚举映射、`#STP` 格式等）+ 局部引用校验
3. **查表无依赖** — 不依赖其他命令的结果

### 局部引用校验

域级判据见 `bms/AGENTS.md` 职责判据。tokenizer 须覆盖三层：

1. **id 格式**——字符集（Base62）+ 长度（1–2 字符）。已由 `BmsIndex::FromStr` 覆盖
2. **id 范围**——某些命令的 id 须落在特定段（如非保留段）
3. **多字段局部一致性**——同一命令内字段间关联校验

## 字符串存储

`BmsToken` / `BmsHeader` / `BmsMessage` 中的字符串字段统一使用 `String` 类型。
`tokenize` 方法始终产出 owned `String` 值：

```rust
let tokens: Vec<(_, _)> = BmsTokenizer::new().tokenize(input);
```

## 新增 Header

在对应的 domain enum 中添加 `#[bms_token("...")]` 变体，derive 自动生成一切。

三层解析：

| 层级 | 机制 | 失败路径 |
|------|------|----------|
| Command match | derive 匹配命令名，提取 `{id}` | 不匹配 → `BmsHeaderFallback` |
| Value parse | derive per-field：`String` / `FromStr` / `BmsValue::parse` | 解析失败 → error（`#[bms_fallback]` 则返回 `None`） |
| Fallback | `#[bms_fallback]` 捕获未匹配 | → `BmsHeaderFallback` |

### Value 类型

- 实现 `BmsValue`（或 `FromStr + Display` —— 有 blanket impl）
- `ParseBmsValueError` / `BmsChannelIdError` / `ParseDifficultyError` 已内置 `IntoTokensError`
- 自定义 `FromStr` 类型可实现 `IntoTokensError` 选择错误变体

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| `BmsIndex` 比较大小写敏感 | 标准 BMS 在 parser 层 normalize 为大写；Base62 模式保留原大小写 |
| `parse_message_line` / `parse_header_line` 对集成测试可见 | 已 re-export，测试可直接调用 |
| `#` + `%` 是默认 header 前缀 | 可通过 `BmsTokenizer::header_prefixes()` 自定义 |
| 三行结尾格式均支持 | LF / CRLF / standalone CR |
| `//` 和 `;` 行首注释 | tokenizer 主循环跳过；inline `//`（前一字符须为空白或行首，含 `"..."` 字符串保护）也由主循环处理；`/* */` 和 `;` 行中注释仍由 `preprocess()` 处理。空白前置规则避免 URL 中的 `//`（如 `https://`）被误判 |

## Always / Ask / Never

### Always

- 新增 header 时在对应 domain enum 加 `#[bms_token("...")]` 变体
- 每新增一个子 enum 变体，检查是否需要 `From<T>` / `TryFrom<T>` 到 parent

### Ask

- 新增 `BmsChannel` 变体（涉及通道分类映射，需确认通道号表）

### Never

- 在 tokenizer 层解析跨命令引用（WAV/BMP 引用语义归 processor；`Bms` 保真存引用形态）
- 修改 `BmsIndex` 的比较语义
