# bms/ — BMS 格式解析管道

## 定位

BMS（Be-Music Script）格式解析管道。`.bms` 原始文本经四阶段管道转换为格式无关的 `Chart<T>`。

## 管道

```mermaid
flowchart LR
    Raw[".bms (&str)"] --> Tok[bms-tokenizer]
    Tok --> CF[bms-control-flow]
    CF --> Par[bms-parser]
    Par --> Proc[bms-processor]
    Proc --> Chart["Chart&lt;T&gt;"]
```

| 阶段 | Crate | 职责 |
|------|-------|------|
| 1 词法 | `bms-tokenizer` | 原始文本 → `BmsToken<C>` |
| — 派生 | `bms-tokenizer-derive` | `#[derive(BmsTokenAttr)]` proc-macro，无运行时逻辑 |
| 2 控制流 | `bms-control-flow` | `#RANDOM`/`#SWITCH` 分支选择与 roundtrip |
| 3 语义 | `bms-parser` | flat token → `Bms` 模型 |
| 4 转换 | `bms-processor` | `Bms` → `Chart` |

## 设计原则

| 原则 | 含义 |
|------|------|
| 分层不越界 | tokenizer 不解析值语义、control-flow 不依赖 parser、parser 不展开控制流、processor 不引入 I/O |
| 泛型贯穿 | `C`（`&str` 零拷贝或 `String` owned）和 `P`（负载类型）由调用侧选择，贯穿管道 |
| 模式族解耦 | 键位映射通过零大小类型实现 `BmsLayout` trait，不在 `Chart` 中存储模式信息 |

## 领域术语

BMS 命令、通道映射、头部字段等术语参照 **bms skill**（`~/.agents/skills/bms/`）。
该 skill 是索引——具体命令语义须查阅其 `memo/`、`ext/`、`bmse/`、`test/` 子目录。

## 测试

```bash
cargo test -p bms-tokenizer
cargo test -p bms-control-flow
cargo test -p bms-parser
cargo test -p bms-processor
# bms-tokenizer-derive 通过 bms-tokenizer 的 roundtrip 测试间接验证
```

## 域级约束

### Never

- 在 BMS 管道 crate 中引入 `serde` / 序列化依赖——序列化属于 bmson 域
