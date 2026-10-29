# bmsrs-bms-syntax

## 定位

BMS 语法层：在解码后的文本上构建 rowan CST，负责词法、容错解析、
控制流嵌套与分支选择查询。不做字节编码（`bmsrs-bms-source`）与语义值解析
（`bmsrs-bms-parser`）。

## 核心不变量

| 不变量 | 含义 |
|--------|------|
| 无损 | 所有字符进入 token，`SyntaxNode::text()` 还原输入 |
| 容错 | 非法行、未闭合块、孤立指令不 panic；错误进 `Diagnostic` |
| 行导向 | 每行至少一个 `Line` 节点；控制流块嵌套在 `SourceFile` 下 |
| 语义无关 | 不构造 `BmsHeader` / `BmsMessage`，只保留命令名与值文本 |
| 分支可选 | CST 保留全部分支；`select_lines` 按随机值返回选中行 |

## 树形

```text
SourceFile
  Line (HeaderLine / MessageLine / CommentLine / BlankLine / TextLine / ErrorLine)
  RandomBlock
    RandomDirective
    RandomChain
      RandomBranch
        IfDirective
        Item*
        ElseIfDirective
        Item*
        ElseDirective
        Item*
        EndIfDirective
    EndRandomDirective
  SwitchBlock
    SwitchDirective
    SwitchCase
      CaseDirective / DefDirective
      Item*
      SkipDirective
    EndSwDirective
```

- 行内 token：`Hash` / `Percent` / `CommandName` / `Value` / `Measure` /
  `Channel` / `Colon` / `BodyChunk`，以及 `Whitespace` / `Comment` / `Newline`
- 非法内容包进 `ErrorLine` 或 `ErrorToken`，尽量保留原文

## API

- `SourceFile::parse(text)` 返回 `Parse`；`Parse::source_file()` / `errors()`
- `SourceFile::parse_with(text, &ParseOptions)` 支持自定义 header 前缀
- `SourceFile::parse_document(&Document)` / `parse_document_with` 从无损文档进入 CST
- `SourceFile::lines()` 深度优先迭代所有行
- `Line::command_name()` / `value()` / `measure()` / `channel()` / `body_chunks()`
- `SourceFile::select_lines(rng)` 返回 `(Vec<Line>, BranchSelection)`；
  `rng(max)` 返回 `[1, max]`，`#SETRANDOM` / `#SETSWITCH` 不调用
- `ParseOptions.header_prefixes` 默认 `['#', '%']`；非默认前缀生成 `Prefix` token

## 控制流选择

| 块 | 规则 |
|----|------|
| `#RANDOM` | 每块一个随机值；每条 `#IF` 链独立首匹配；`#ELSE` 兜底 |
| `#SETRANDOM` | 使用固定值，不调用 RNG |
| `#SWITCH` | 每块一个随机值；首个匹配 `#CASE` 后 fall-through，直到带 `#SKIP` 的 case |
| `#DEF` | 总是匹配 |
| 0 值 | `#RANDOM 0` / `#SWITCH 0` 视为无匹配，不 panic |

## 测试

```bash
cargo test -p bmsrs-bms-syntax
```

公开 API 测试放 `tests/`，覆盖：lossless、行内 token、控制流嵌套、
错误恢复、分支选择。新增语法节点时补一条 lossless 断言。

## Always / Ask / Never

### Always

- 新 token/节点先加 `SyntaxKind`，再在 lexer/parser/builder 三处同步
- 保持每个字符只进入一个 token；用 `SyntaxNode::text()` 做回归
- 控制流指令行只做结构节点，不塞进 payload

### Ask

- 修改控制流选择语义（fall-through、`#SKIP`、多链）
- 修改树形（扁平化、合并节点）

### Never

- 在语法层做编码检测或字节写回
- 在语法层构造 `BmsHeader` / `BmsMessage` 语义值
- 对非法输入 panic 或提前返回空树
