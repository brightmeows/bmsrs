# bms-values

## 定位

BMS 单行语法与类型化值：header/message、索引类型、单行解析与编码。
不持有整文件循环，也不构建 CST；`bms-syntax` 负责整文件结构，
`bms-parser` 负责 CST → `Bms`，`bms-analysis` 直接复用本 crate 的值类型。

## 关键边界

| 做 | 不做 |
|----|------|
| 单行命令匹配与值解析 | 分行、注释状态机、编码检测 |
| 类型化 `BmsHeader` / `BmsMessage` | 控制流展开与分支选择 |
| Base36 / Base62 索引与归一化 | `Bms` 聚合模型 |
| 单行 roundtrip（解析 ↔ 格式化） | 整文件写回 |

## 三层解析

| 层级 | 机制 | 失败路径 |
|------|------|----------|
| Command match | derive 匹配命令名，提取 `{id}` | 不匹配 → `BmsHeaderFallback` |
| Value parse | `String` / `FromStr` / `BmsValue::parse` | 失败 → `BmsTokenizeError`；`#[bms_fallback]` 返回 `None` |
| Fallback | `#[bms_fallback]` 捕获未匹配 | → `BmsHeaderFallback` |

## 非显而易见规则

| 规则 | 说明 |
|------|------|
| `BmsIndex` 比较大小写敏感 | 标准 BMS 在 parser 层 normalize 为大写；Base62 保留原大小写 |
| `##` 开头不是头部 | `parse_header_line` 返回 `Ok(None)`，由语法层分类为注释 |
| `#` + `%` 前缀由调用方提供 | 默认值 `['#', '%']` 由调用方传入 |
| 值类型 owned | 单行入口不借用输入，便于跨层传递 |

## 测试

```bash
cargo test -p bms-values
```

覆盖：命令匹配、值解析、索引、通道分类、单行 roundtrip、derive。

## Always / Ask / Never

### Always

- 新增 header 时在对应 domain enum 加 `#[bms_token("...")]` 变体
- 每新增一个子 enum 变体，检查是否需要 `From<T>` / `TryFrom<T>` 到 parent
- 新增值类型时实现 `BmsValue`（或 `FromStr + Display`）

### Ask

- 新增 `BmsChannel` 变体（涉及通道分类映射）
- 修改 `BmsIndex` 的比较语义

### Never

- 在这里做整文件分词、编码检测或控制流展开
- 解析跨命令引用（WAV/BMP 引用语义归 processor）
- 引入 `bms-syntax` / `bms-parser` 依赖，保持本 crate 单向被依赖
