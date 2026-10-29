# bmsrs-bms-source

## 定位

BMS 源文本层：原始字节 → 行 → 编辑 → 写回。只处理“字节到行”，不解析
BMS 语法；头部、通道与控制流由 `bmsrs-bms-syntax` 的 CST 负责。

## 核心不变量

| 不变量 | 含义 |
|--------|------|
| 未编辑行字节不变 | `render()` 对未编辑行直接切原始字节，不经过解码与重新编码 |
| 只编码被编辑的行 | 结构化编辑只替换内容段，前缀/后缀 trivia 按原始字节拼接 |
| 行终止符逐行保留 | LF / CRLF / CR / 无终止符各自保留；插入行继承相邻行 |
| BOM 单独保留 | BOM 是文档前缀，不进入首行内容 |
| 不可表示字符显式失败 | `EditError::Unmappable`，文档不变，不做替换 |
| 不做语义分类 | 行只分为 `Blank` / `Comment` / `Text` / `Error(Decode)` |

## 行模型

```text
<前缀 trivia><内容段><后缀 trivia><行尾终止符>
```

- 内容段是第一个连续的非 trivia 区段；`Line::content` 记录它在原始字节中的区间
- 前缀与后缀包含缩进、行内 `//` 注释与跨行 `/* */` 块注释的残余
- `LineKind`：`Blank` / `Comment` / `Text` / `Error`

## 文本桥

- `Document::to_text()`：去 BOM、保留原行尾，解码错误用 replacement
- `Document::try_to_text()`：严格版，返回 `TextDecodeError`
- `Document::render_line_indices(&[usize])`：0-based 选行，含 BOM，未编辑行原样切片

## 注释状态机

状态机在**原始字节**上扫描，不在解码后的文本上。依据是：所有受支持的
ASCII 兼容编码中，注释标记（`/`、`*`、`;`、`"`）与行终止符都不会出现在
多字节字符的尾字节中（`Shift_JIS` / `GBK` / `Big5` 尾字节从 `0x40` 起，
`EUC-KR` / `EUC-JP` 从 `0xA1` 起）。UTF-16 按 2 字节代码单元扫描。

| 语法 | 规则 |
|------|------|
| `//` | 前一字符为空白或行首时开启行注释；字符串内不生效 |
| `;` | 仅行首（允许前置空白）开启整行注释 |
| `/* */` | 不嵌套，可跨行；内容段在首个注释处冻结 |
| `"..."` | 反斜杠转义；字符串状态跨行保留 |

## 编码

- 支持全部 ASCII 兼容且无状态的编码与 UTF-16；拒绝 `ISO-2022-JP`（跨行状态）与 `replacement`（不可解码）
- 检测顺序：BOM → `#CHARSET` → 严格 UTF-8 → `Shift_JIS` / `EUC-KR` / `GBK` / `Big5`
- `#CHARSET` 声明解码失败时回退候选表，`DetectionMethod::DeclaredFallback` 回报
- 编码被编辑行时检查 `Encoding::encode` 返回的 `had_unmappable` 标志；该函数会把不可映射字符替换成数字字符引用，标志为真时丢弃结果并返回错误

## 测试

```bash
cargo test -p bmsrs-bms-source
```

公开 API 测试放 `tests/`，覆盖：字节 roundtrip（各编码 × 各终止符）、
行分类、注释状态机、行编辑、编码检测。

## Always / Ask / Never

### Always

- 新增编码支持前先确认它 ASCII 兼容且无状态，或单独实现分行策略
- 编辑路径必须保证失败时文档不变
- 新增行分类只做字节/trivia 层面的判断；语义分类留给 `bmsrs-bms-syntax`

### Ask

- 修改字节保真不变量（例如引入整文件重编码）
- 修改注释语义（会影响所有使用方的 token 边界）

### Never

- 在源层做编码转换、行尾统一或 BOM 增删
- 用 `Encoding::encode` 直接写回被编辑行
- 在未编辑行上重新编码
- 在本 crate 引入 BMS tokenizer / parser / control-flow 依赖
