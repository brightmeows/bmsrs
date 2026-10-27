//! 容错解析：把行 token 组装为 rowan CST。

use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

use crate::SyntaxKind;
use crate::ast::SourceFile;
use crate::lexer::{LineData, LineKind, lex};

/// 诊断类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// 语法错误。
    Syntax,
    /// 控制流结构错误。
    ControlFlow,
}

/// 一条解析诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 文本范围。
    range: TextRange,
    /// 诊断消息。
    message: String,
    /// 诊断类别。
    kind: DiagnosticKind,
}

impl Diagnostic {
    /// 构造诊断。
    fn new(range: TextRange, message: impl Into<String>, kind: DiagnosticKind) -> Self {
        Self {
            range,
            message: message.into(),
            kind,
        }
    }

    /// 返回文本范围。
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    /// 返回诊断消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 返回诊断类别。
    #[must_use]
    pub const fn kind(&self) -> DiagnosticKind {
        self.kind
    }
}

/// 一次解析的结果。
#[derive(Debug, Clone)]
pub struct Parse {
    /// 绿色树。
    green: GreenNode,
    /// 诊断列表。
    errors: Vec<Diagnostic>,
}

impl Parse {
    /// 返回文档根节点。
    #[must_use]
    pub fn source_file(&self) -> SourceFile {
        SourceFile(self.syntax())
    }

    /// 返回新建的红色根节点。
    #[must_use]
    pub fn syntax(&self) -> crate::SyntaxNode {
        crate::SyntaxNode::new_root(self.green.clone())
    }

    /// 返回诊断列表。
    #[must_use]
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }
}

/// CST 条目。
enum Item<'a> {
    /// 普通行。
    Line(LineData<'a>),
    /// `#RANDOM` 块。
    Random(RandomBlock<'a>),
    /// `#SWITCH` 块。
    Switch(SwitchBlock<'a>),
}

/// `#RANDOM` 块。
struct RandomBlock<'a> {
    /// 块起始指令。
    directive: LineData<'a>,
    /// 指令与分支之外的条目。
    body: Vec<Item<'a>>,
    /// 互斥链。
    chains: Vec<RandomChain<'a>>,
    /// `#ENDRANDOM`。
    end: Option<LineData<'a>>,
}

/// `#IF` 互斥链。
struct RandomChain<'a> {
    /// 分支列表。
    branches: Vec<RandomBranch<'a>>,
    /// `#ENDIF`。
    end: Option<LineData<'a>>,
}

/// 互斥链中的单个分支。
struct RandomBranch<'a> {
    /// 分支起始指令。
    directive: LineData<'a>,
    /// 分支内容。
    body: Vec<Item<'a>>,
}

/// `#SWITCH` 块。
struct SwitchBlock<'a> {
    /// 块起始指令。
    directive: LineData<'a>,
    /// case 之外的条目。
    body: Vec<Item<'a>>,
    /// case 列表。
    cases: Vec<SwitchCase<'a>>,
    /// `#ENDSW`。
    end: Option<LineData<'a>>,
}

/// `#SWITCH` 的单个 case。
struct SwitchCase<'a> {
    /// case 起始指令。
    directive: LineData<'a>,
    /// case 内容。
    body: Vec<Item<'a>>,
    /// `#SKIP`。
    skip: Option<LineData<'a>>,
}

/// 解析文本，构建 CST。
pub fn parse(text: &str) -> Parse {
    let lines = lex(text);
    let mut errors = Vec::new();
    collect_lex_errors(&lines, &mut errors);
    let items = parse_items(&lines, 0, lines.len(), &mut errors);

    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::SourceFile.raw());
    build_items(&mut builder, &items);
    builder.finish_node();

    Parse {
        green: builder.finish(),
        errors,
    }
}

/// 收集词法层的错误行。
fn collect_lex_errors(lines: &[LineData<'_>], errors: &mut Vec<Diagnostic>) {
    for line in lines {
        if line.kind == LineKind::Error {
            let range = line
                .tokens
                .iter()
                .find(|token| token.kind == SyntaxKind::ErrorToken)
                .map_or_else(
                    || line_range(line),
                    |token| TextRange::new(text_size(token.start), text_size(token.end())),
                );
            errors.push(Diagnostic::new(
                range,
                "invalid BMS line",
                DiagnosticKind::Syntax,
            ));
        }
    }
}

/// 解析一段行范围。
fn parse_items<'a>(
    lines: &[LineData<'a>],
    start: usize,
    end: usize,
    errors: &mut Vec<Diagnostic>,
) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let mut index = start;
    while index < end {
        let (item, next) = parse_item(lines, index, end, errors);
        items.push(item);
        index = next;
    }
    items
}

/// 解析单个条目。
fn parse_item<'a>(
    lines: &[LineData<'a>],
    index: usize,
    end: usize,
    errors: &mut Vec<Diagnostic>,
) -> (Item<'a>, usize) {
    let Some(line) = lines.get(index) else {
        return (Item::Line(empty_line()), end);
    };

    if is_random_open(line) {
        parse_random(lines, index, end, errors)
    } else if is_switch_open(line) {
        parse_switch(lines, index, end, errors)
    } else if is_control_directive(line) {
        errors.push(Diagnostic::new(
            line_range(line),
            "unmatched control-flow directive",
            DiagnosticKind::ControlFlow,
        ));
        let mut error = line.clone();
        error.kind = LineKind::Error;
        (Item::Line(error), index.saturating_add(1))
    } else {
        (Item::Line(line.clone()), index.saturating_add(1))
    }
}

/// 构造空行，仅用于越界兜底。
const fn empty_line<'a>() -> LineData<'a> {
    LineData {
        kind: LineKind::Blank,
        tokens: Vec::new(),
        start: 0,
        end: 0,
    }
}

/// 解析一个 `#RANDOM` 块。
fn parse_random<'a>(
    lines: &[LineData<'a>],
    start: usize,
    end: usize,
    errors: &mut Vec<Diagnostic>,
) -> (Item<'a>, usize) {
    let end_index = find_end(lines, start, end, is_random_open, is_end_random);
    let inner_end = end_index.unwrap_or(end);
    let (body, chains) = parse_random_inner(lines, start.saturating_add(1), inner_end, errors);
    let end_line = end_index.and_then(|index| lines.get(index).cloned());
    if end_index.is_none()
        && let Some(line) = lines.get(start)
    {
        errors.push(Diagnostic::new(
            line_range(line),
            "unclosed #RANDOM",
            DiagnosticKind::ControlFlow,
        ));
    }
    let next = end_index.map_or(inner_end, |index| index.saturating_add(1));
    (
        Item::Random(RandomBlock {
            directive: lines.get(start).cloned().unwrap_or_else(empty_line),
            body,
            chains,
            end: end_line,
        }),
        next,
    )
}

/// 解析 `#RANDOM` 内部。
fn parse_random_inner<'a>(
    lines: &[LineData<'a>],
    start: usize,
    end: usize,
    errors: &mut Vec<Diagnostic>,
) -> (Vec<Item<'a>>, Vec<RandomChain<'a>>) {
    let mut body = Vec::new();
    let mut chains = Vec::new();
    let mut current_chain: Option<RandomChain<'a>> = None;
    let mut current_branch: Option<RandomBranch<'a>> = None;
    let mut index = start;

    while index < end {
        let Some(line) = lines.get(index) else {
            break;
        };
        if is_command(line, "IF") {
            close_chain(
                &mut chains,
                &mut current_chain,
                &mut current_branch,
                errors,
                line,
            );
            current_chain = Some(RandomChain {
                branches: Vec::new(),
                end: None,
            });
            current_branch = Some(RandomBranch {
                directive: line.clone(),
                body: Vec::new(),
            });
            index = index.saturating_add(1);
        } else if is_command(line, "ELSEIF") || is_command(line, "ELSE") {
            if let Some(mut chain) = current_chain.take() {
                if let Some(branch) = current_branch.take() {
                    chain.branches.push(branch);
                }
                current_chain = Some(chain);
                current_branch = Some(RandomBranch {
                    directive: line.clone(),
                    body: Vec::new(),
                });
            } else {
                push_error(
                    &mut body,
                    &mut current_branch,
                    line,
                    "control-flow branch outside #RANDOM",
                    errors,
                );
            }
            index = index.saturating_add(1);
        } else if is_command(line, "ENDIF") || is_command(line, "IFEND") {
            if let Some(mut chain) = current_chain.take() {
                if let Some(branch) = current_branch.take() {
                    chain.branches.push(branch);
                }
                chain.end = Some(line.clone());
                chains.push(chain);
            } else {
                push_error(
                    &mut body,
                    &mut current_branch,
                    line,
                    "#ENDIF without #IF",
                    errors,
                );
            }
            index = index.saturating_add(1);
        } else if is_end_random(line) {
            break;
        } else {
            let (item, next) = parse_item(lines, index, end, errors);
            push_item(&mut body, &mut current_branch, item);
            index = next;
        }
    }

    if let Some(mut chain) = current_chain.take() {
        if let Some(branch) = current_branch.take() {
            chain.branches.push(branch);
        }
        if chain.end.is_none()
            && let Some(line) = chain.branches.first().map(|branch| &branch.directive)
        {
            errors.push(Diagnostic::new(
                line_range(line),
                "#IF without #ENDIF",
                DiagnosticKind::ControlFlow,
            ));
        }
        chains.push(chain);
    }

    (body, chains)
}

/// 关闭当前互斥链。
fn close_chain<'a>(
    chains: &mut Vec<RandomChain<'a>>,
    current_chain: &mut Option<RandomChain<'a>>,
    current_branch: &mut Option<RandomBranch<'a>>,
    errors: &mut Vec<Diagnostic>,
    line: &LineData<'a>,
) {
    if let Some(mut chain) = current_chain.take() {
        if let Some(branch) = current_branch.take() {
            chain.branches.push(branch);
        }
        errors.push(Diagnostic::new(
            line_range(line),
            "previous #IF chain missing #ENDIF",
            DiagnosticKind::ControlFlow,
        ));
        chains.push(chain);
    }
}

/// 解析一个 `#SWITCH` 块。
fn parse_switch<'a>(
    lines: &[LineData<'a>],
    start: usize,
    end: usize,
    errors: &mut Vec<Diagnostic>,
) -> (Item<'a>, usize) {
    let end_index = find_end(lines, start, end, is_switch_open, is_end_switch);
    let inner_end = end_index.unwrap_or(end);
    let (body, cases) = parse_switch_inner(lines, start.saturating_add(1), inner_end, errors);
    let end_line = end_index.and_then(|index| lines.get(index).cloned());
    if end_index.is_none()
        && let Some(line) = lines.get(start)
    {
        errors.push(Diagnostic::new(
            line_range(line),
            "unclosed #SWITCH",
            DiagnosticKind::ControlFlow,
        ));
    }
    let next = end_index.map_or(inner_end, |index| index.saturating_add(1));
    (
        Item::Switch(SwitchBlock {
            directive: lines.get(start).cloned().unwrap_or_else(empty_line),
            body,
            cases,
            end: end_line,
        }),
        next,
    )
}

/// 解析 `#SWITCH` 内部。
fn parse_switch_inner<'a>(
    lines: &[LineData<'a>],
    start: usize,
    end: usize,
    errors: &mut Vec<Diagnostic>,
) -> (Vec<Item<'a>>, Vec<SwitchCase<'a>>) {
    let mut body = Vec::new();
    let mut cases = Vec::new();
    let mut current_case: Option<SwitchCase<'a>> = None;
    let mut index = start;

    while index < end {
        let Some(line) = lines.get(index) else {
            break;
        };
        if is_command(line, "CASE") || is_command(line, "DEF") {
            if let Some(case) = current_case.take() {
                cases.push(case);
            }
            current_case = Some(SwitchCase {
                directive: line.clone(),
                body: Vec::new(),
                skip: None,
            });
            index = index.saturating_add(1);
        } else if is_command(line, "SKIP") {
            if let Some(case) = current_case.as_mut() {
                case.skip = Some(line.clone());
            } else {
                body.push(Item::Line(error_line(line, "#SKIP outside #CASE", errors)));
            }
            index = index.saturating_add(1);
        } else if is_end_switch(line) {
            break;
        } else {
            let (item, next) = parse_item(lines, index, end, errors);
            if let Some(case) = current_case.as_mut() {
                case.body.push(item);
            } else {
                body.push(item);
            }
            index = next;
        }
    }

    if let Some(case) = current_case.take() {
        cases.push(case);
    }

    (body, cases)
}

/// 找到匹配的块结束行。
fn find_end(
    lines: &[LineData<'_>],
    start: usize,
    end: usize,
    is_open: fn(&LineData<'_>) -> bool,
    is_close: fn(&LineData<'_>) -> bool,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = start.saturating_add(1);
    while index < end {
        let Some(line) = lines.get(index) else {
            break;
        };
        if is_open(line) {
            depth = depth.saturating_add(1);
        } else if is_close(line) {
            if depth == 0 {
                return Some(index);
            }
            depth = depth.saturating_sub(1);
        }
        index = index.saturating_add(1);
    }
    None
}

/// 把条目追加到当前分支或块体。
fn push_item<'a>(body: &mut Vec<Item<'a>>, branch: &mut Option<RandomBranch<'a>>, item: Item<'a>) {
    if let Some(current) = branch {
        current.body.push(item);
    } else {
        body.push(item);
    }
}

/// 追加一条错误行。
fn push_error<'a>(
    body: &mut Vec<Item<'a>>,
    branch: &mut Option<RandomBranch<'a>>,
    line: &LineData<'a>,
    message: &str,
    errors: &mut Vec<Diagnostic>,
) {
    push_item(body, branch, Item::Line(error_line(line, message, errors)));
}

/// 把一行标记为错误并记录诊断。
fn error_line<'a>(
    line: &LineData<'a>,
    message: &str,
    errors: &mut Vec<Diagnostic>,
) -> LineData<'a> {
    errors.push(Diagnostic::new(
        line_range(line),
        message,
        DiagnosticKind::ControlFlow,
    ));
    let mut error = line.clone();
    error.kind = LineKind::Error;
    error
}

/// 迭代构建 CST。
fn build_items(builder: &mut GreenNodeBuilder<'_>, items: &[Item<'_>]) {
    for item in items {
        match item {
            Item::Line(line) => build_line(builder, line),
            Item::Random(block) => build_random(builder, block),
            Item::Switch(block) => build_switch(builder, block),
        }
    }
}

/// 构建普通行。
fn build_line(builder: &mut GreenNodeBuilder<'_>, line: &LineData<'_>) {
    let kind = match line.kind {
        LineKind::Blank => SyntaxKind::BlankLine,
        LineKind::Comment => SyntaxKind::CommentLine,
        LineKind::Header => SyntaxKind::HeaderLine,
        LineKind::Message => SyntaxKind::MessageLine,
        LineKind::Text => SyntaxKind::TextLine,
        LineKind::Error => SyntaxKind::ErrorLine,
    };
    build_node(builder, line, kind);
}

/// 构建带 token 的节点。
fn build_node(builder: &mut GreenNodeBuilder<'_>, line: &LineData<'_>, kind: SyntaxKind) {
    builder.start_node(kind.raw());
    for token in &line.tokens {
        builder.token(token.kind.raw(), token.text);
    }
    builder.finish_node();
}

/// 构建 `#RANDOM` 块。
fn build_random(builder: &mut GreenNodeBuilder<'_>, block: &RandomBlock<'_>) {
    builder.start_node(SyntaxKind::RandomBlock.raw());
    build_node(builder, &block.directive, SyntaxKind::RandomDirective);
    build_items(builder, &block.body);
    for chain in &block.chains {
        builder.start_node(SyntaxKind::RandomChain.raw());
        for branch in &chain.branches {
            builder.start_node(SyntaxKind::RandomBranch.raw());
            build_node(builder, &branch.directive, branch_kind(&branch.directive));
            build_items(builder, &branch.body);
            builder.finish_node();
        }
        if let Some(end) = &chain.end {
            build_node(builder, end, SyntaxKind::EndIfDirective);
        }
        builder.finish_node();
    }
    if let Some(end) = &block.end {
        build_node(builder, end, SyntaxKind::EndRandomDirective);
    }
    builder.finish_node();
}

/// 构建 `#SWITCH` 块。
fn build_switch(builder: &mut GreenNodeBuilder<'_>, block: &SwitchBlock<'_>) {
    builder.start_node(SyntaxKind::SwitchBlock.raw());
    build_node(builder, &block.directive, SyntaxKind::SwitchDirective);
    build_items(builder, &block.body);
    for case in &block.cases {
        builder.start_node(SyntaxKind::SwitchCase.raw());
        build_node(builder, &case.directive, case_kind(&case.directive));
        build_items(builder, &case.body);
        if let Some(skip) = &case.skip {
            build_node(builder, skip, SyntaxKind::SkipDirective);
        }
        builder.finish_node();
    }
    if let Some(end) = &block.end {
        build_node(builder, end, SyntaxKind::EndSwDirective);
    }
    builder.finish_node();
}

/// 返回分支指令的节点种类。
fn branch_kind(line: &LineData<'_>) -> SyntaxKind {
    if is_command(line, "IF") {
        SyntaxKind::IfDirective
    } else if is_command(line, "ELSEIF") {
        SyntaxKind::ElseIfDirective
    } else {
        SyntaxKind::ElseDirective
    }
}

/// 返回 case 指令的节点种类。
fn case_kind(line: &LineData<'_>) -> SyntaxKind {
    if is_command(line, "DEF") {
        SyntaxKind::DefDirective
    } else {
        SyntaxKind::CaseDirective
    }
}

/// 返回行的文本范围。
fn line_range(line: &LineData<'_>) -> TextRange {
    TextRange::new(text_size(line.start), text_size(line.end))
}

/// 把字节偏移转为 `TextSize`。
fn text_size(offset: usize) -> TextSize {
    TextSize::from(u32::try_from(offset).unwrap_or(u32::MAX))
}

/// 是否是 `#RANDOM` / `#SETRANDOM`。
fn is_random_open(line: &LineData<'_>) -> bool {
    is_command(line, "RANDOM") || is_command(line, "SETRANDOM") || is_command(line, "RONDAM")
}

/// 是否是 `#ENDRANDOM`。
fn is_end_random(line: &LineData<'_>) -> bool {
    is_command(line, "ENDRANDOM")
}

/// 是否是 `#SWITCH` / `#SETSWITCH`。
fn is_switch_open(line: &LineData<'_>) -> bool {
    is_command(line, "SWITCH") || is_command(line, "SETSWITCH")
}

/// 是否是 `#ENDSW`。
fn is_end_switch(line: &LineData<'_>) -> bool {
    is_command(line, "ENDSW") || is_command(line, "ENDSWITCH")
}

/// 是否是任意控制流指令。
fn is_control_directive(line: &LineData<'_>) -> bool {
    [
        "RANDOM",
        "SETRANDOM",
        "RONDAM",
        "IF",
        "ELSEIF",
        "ELSE",
        "ENDIF",
        "IFEND",
        "ENDRANDOM",
        "SWITCH",
        "SETSWITCH",
        "CASE",
        "SKIP",
        "DEF",
        "ENDSW",
        "ENDSWITCH",
    ]
    .iter()
    .any(|command| is_command(line, command))
}

/// 判断命令名是否匹配。
fn is_command(line: &LineData<'_>, command: &str) -> bool {
    line.command_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(command))
}
