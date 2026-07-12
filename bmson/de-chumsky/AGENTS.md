# bmson-de-chumsky

## 定位

chumsky 驱动的 BMSON 反序列化。JSON 文本 → `bmson_def` 类型。

## 两阶段设计

| 阶段 | 引擎 | 作用 |
|------|------|------|
| 1. 校验 | chumsky JSON parser | 语法校验 + 错误恢复，产出 `serde_json::Value` |
| 2. 反序列化 | `serde_json::from_str` | 零拷贝反序列化为 `bmson_def::Bmson` |

chumsky 阶段产出的诊断信息在 serde_json 失败时附加到错误中。

## 入口

`BmsonParser` 是零大小类型，通过关联方法 `parse` 暴露全部功能：

```rust
let bmson = BmsonParser::parse(json)?;  // → bmson_def::Bmson<'_>
```

## 版本分派

`parse` 内部根据 `bmson_def::DetectedVersion` 分派到版本特定反序列化：

| 版本 | 路径 |
|------|------|
| v2 | 直接 `serde_json::from_str` |
| v1 | 反序列化为 `v1::Bmson` → `Bmson::from` 升版 |
| v0 | 反序列化为 `v0::Bmson` → `Bmson::try_from` 升版 |

## 错误恢复

chumsky JSON parser 支持错误恢复——尾随逗号、缺失逗号、括号不匹配会产生警告但仍继续解析。

原始 parser 可通过 `json::parse_json` 直接使用，返回 `(Option<Value>, Vec<ParseError>)`。
适合需要对 JSON 解析进行细粒度控制的场景。

## 模块结构

| 文件 | 职责 |
|------|------|
| `lib.rs` | `BmsonParser` 入口 + 版本分派 |
| `json.rs` | chumsky JSON parser + 错误分类 |
| `error.rs` | `BmsonDeError` 错误类型 |

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| chumsky 产出 `None` = 致命错误 | 无论错误如何分类，无输出即返回 `JsonParse` 错误 |
| 错误分三级 | `classify_errors` 划分 Warning（`Rich::custom`）/ Recovered（有输出时的 `ExpectedFound`）/ Fatal（无输出时） |
| 反序列化失败附带诊断 | serde_json 失败时，chumsky 诊断信息附加到 `Deserialize` 错误消息 |
| 返回值零拷贝 | `Bmson<'_>` 从输入 `json: &str` 借用 |
