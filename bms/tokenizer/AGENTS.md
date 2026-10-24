# bms-tokenizer

## 定位

BMS 单行语法：把**一行**文本解析为 `BmsToken`，并提供 header 声明表。

整文件的分行、注释状态机、编码检测与行级编辑归 `bms-document`；本 crate
不持有任何整文件循环。

## 关键设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 通道分类时机 | tokenizer 阶段 | 下游 parser 无需重复解析通道号 |
| 跨命令引用保留 | 不解析跨命令引用，保留 raw index | 引用语义（WAV→定义、`#BGA`→`#BMP`）归 processor；`#BASE` 模式由 parser 决定 |
| 字符集 | Base62（最宽松） | 统一入口，`#BASE` 声明在 parser 层生效 |
| 注释与编码 | 不处理 | 归 `bms-document`：注释状态机、BOM、行尾与编码检测都在那里 |
| 整文件循环 | 不提供 | `Document` 是唯一整文件入口，避免两份分行规则 |

## 目标

每个 token 的存在只为三个目的：

1. **Roundtrip 保真** — `format_header` 还原命令与值
2. **单行验证** — 语法 + 单行值解释（`Rank` 枚举映射、`#STP` 格式等）+ 局部引用校验
3. **查表无依赖** — 不依赖其他命令的结果

### 局部引用校验

域级判据见 `bms/AGENTS.md` 职责判据。tokenizer 须覆盖三层：

1. **id 格式**——字符集（Base62）+ 长度（1–2 字符）。已由 `BmsIndex::FromStr` 覆盖
2. **id 范围**——某些命令的 id 须落在特定段（如非保留段）
3. **多字段局部一致性**——同一命令内字段间关联校验

## 字符串存储

`BmsToken` / `BmsHeader` / `BmsMessage` 中的字符串字段统一使用 `String` 类型。
单行解析入口 `parse_header_line` / `parse_message_line` 产出 owned 值，
不借用输入。

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
| `##` 开头的行不是头部 | `parse_header_line` 返回 `Ok(None)`；`bms-document` 将其分类为注释 |
| `#` + `%` 前缀由调用方提供 | 前缀列表是 `parse_header_line` 的参数，默认值在 `DocumentOptions` |

## Always / Ask / Never

### Always

- 新增 header 时在对应 domain enum 加 `#[bms_token("...")]` 变体
- 每新增一个子 enum 变体，检查是否需要 `From<T>` / `TryFrom<T>` 到 parent

### Ask

- 新增 `BmsChannel` 变体（涉及通道分类映射，需确认通道号表）

### Never

- 在 tokenizer 层解析跨命令引用（WAV/BMP 引用语义归 processor；`Bms` 保真存引用形态）
- 修改 `BmsIndex` 的比较语义
- 在这里重新引入整文件分词循环或编码检测
