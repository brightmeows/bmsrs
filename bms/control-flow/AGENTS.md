# bms-control-flow

## 定位

`#RANDOM`/`#SWITCH` 分支选择与 roundtrip。

## 核心模型

`FlowDoc<P>` 携带泛型负载类型 `P`。连续的非控制流 token 打包为单个 payload 节点；控制流命令成为结构化 `FlowBlock` 节点。

```rust
let tree = FlowDoc::from_tokens(tokens)?;
let (flat, sel) = tree.select_branches(&mut rng);
let flat2 = tree.to_tokens();
```

## Payload 泛型

| 类型 | 用途 | 可编辑？ | 可 roundtrip？ |
|------|------|----------|----------------|
| `FlowDoc<TokenPayload>` | token 级真相源 | ✅ | ✅ |
| `FlowDoc<Bms>`（下游构建） | parser 级只读视图 | ❌ | ❌ |

通过 `map_payload` / `try_map_payload` 派生其他 payload 视图，同时保留控制流骨架。

## 分支选择

| 命令 | 选择策略 | 无匹配时 |
|------|----------|----------|
| `#RANDOM n` | 首匹配分支 | 静默空（无错误）|
| `#SWITCH n` | 首匹配 case，fall-through 到 `#SKIP` | 同左 |

- `BranchValue::Max(n)` → 调用 RNG
- `BranchValue::Set(n)` → 固定值（用于 `#SETRANDOM`/`#SETSWITCH`）

## 嵌套规则

| 场景 | 行为 |
|------|------|
| `#IF`/`#ENDIF` | 路由到最近的 `RandomBlock` |
| `#CASE`/`#SKIP` | 路由到最近的 `SwitchBlock` |
| 分支内开新块（如 `#SWITCH` 在 `#IF 1` 内）| 保持包含 |
| 缺失 `#ENDRANDOM` | 内容提升到顶层 + 记录 ControlFlowWarning |
| 开嵌套块前 | 刷新当前作用域待定 token 到 payload 节点 |

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| 独立于 `bms-parser` | 本 crate 不依赖 parser；`FlowDoc<Bms>` 由下游构建 |
| `#IF`…`#ENDIF` 互斥链 | `#IF`/`#ELSEIF`/`#ELSE` 共享单个 `#ENDIF`，建模为 `RandomChain`；一个 `#RANDOM` 块可含多条独立链，选择时各链按首匹配互斥执行 |
| `#SWITCH` 中的 `#SKIP` | fall-through 控制；无 `#SKIP` = 继续执行下一个 case |
| `SequenceRng` | 测试用确定性 RNG，预定义返回值序列 |
| `StdRng` 或 `ThreadRng` | 自动满足 `BranchRng`（blanket impl）|

## 测试

`SequenceRng`、`find_seed`、`build_doc` 定义在测试文件中。
`map_payload_tests` 覆盖 payload 转换和骨架保留。
