//! 控制流分支选择查询。

use crate::{Line, SourceFile, SyntaxKind, SyntaxNode};

/// 单个控制流块的决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDecision {
    /// 使用的随机值。
    value: u64,
    /// 选中的分支/case 索引；无匹配时为分支数。
    selected_index: usize,
}

impl BlockDecision {
    /// 返回随机值。
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }

    /// 返回选中的分支/case 索引。
    #[must_use]
    pub const fn selected_index(self) -> usize {
        self.selected_index
    }
}

/// 所有控制流块的决策。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchSelection {
    /// 按遍历顺序记录的决策。
    decisions: Vec<BlockDecision>,
}

impl BranchSelection {
    /// 返回决策列表。
    #[must_use]
    pub fn decisions(&self) -> &[BlockDecision] {
        &self.decisions
    }
}

impl SourceFile {
    /// 按 `next` 提供的随机值选择分支，返回选中的行。
    ///
    /// `next(max)` 应返回 `[1, max]` 内的值；`#SETRANDOM` / `#SETSWITCH`
    /// 不调用它。
    #[must_use]
    pub fn select_lines(&self, mut next: impl FnMut(u64) -> u64) -> (Vec<Line>, BranchSelection) {
        let mut selection = BranchSelection::default();
        let mut lines = Vec::new();
        select_children(self.syntax(), &mut next, &mut selection, &mut lines);
        (lines, selection)
    }
}

/// 选择父节点下的所有条目。
fn select_children(
    parent: &SyntaxNode,
    next: &mut impl FnMut(u64) -> u64,
    selection: &mut BranchSelection,
    lines: &mut Vec<Line>,
) {
    for child in parent.children() {
        select_node(&child, next, selection, lines);
    }
}

/// 选择单个节点。
fn select_node(
    node: &SyntaxNode,
    next: &mut impl FnMut(u64) -> u64,
    selection: &mut BranchSelection,
    lines: &mut Vec<Line>,
) {
    match node.kind() {
        SyntaxKind::RandomBlock => select_random(node, next, selection, lines),
        SyntaxKind::SwitchBlock => select_switch(node, next, selection, lines),
        SyntaxKind::RandomChain => {
            select_chain(node, 0, next, selection, lines);
        }
        SyntaxKind::RandomBranch | SyntaxKind::SwitchCase => {
            select_body(node, next, selection, lines);
        }
        kind if kind.is_directive() => {}
        _ => {
            if let Some(line) = Line::cast(node.clone()) {
                lines.push(line);
            } else {
                select_children(node, next, selection, lines);
            }
        }
    }
}

/// 选择 `#RANDOM` 块。
fn select_random(
    node: &SyntaxNode,
    next: &mut impl FnMut(u64) -> u64,
    selection: &mut BranchSelection,
    lines: &mut Vec<Line>,
) {
    let value = block_value(node, next);
    let mut first_selected = 0usize;
    let mut chain_index = 0usize;

    for child in node.children() {
        match child.kind() {
            SyntaxKind::RandomChain => {
                let selected = select_chain(&child, value, next, selection, lines);
                if chain_index == 0 {
                    first_selected = selected;
                }
                chain_index = chain_index.saturating_add(1);
            }
            SyntaxKind::RandomDirective | SyntaxKind::EndRandomDirective => {}
            _ => select_node(&child, next, selection, lines),
        }
    }

    selection.decisions.push(BlockDecision {
        value,
        selected_index: first_selected,
    });
}

/// 选择一条互斥链。
fn select_chain(
    chain: &SyntaxNode,
    value: u64,
    next: &mut impl FnMut(u64) -> u64,
    selection: &mut BranchSelection,
    lines: &mut Vec<Line>,
) -> usize {
    let branches: Vec<_> = chain
        .children()
        .filter(|child| child.kind() == SyntaxKind::RandomBranch)
        .collect();
    let selected = branches
        .iter()
        .position(|branch| branch_matches(branch, value))
        .unwrap_or(branches.len());
    if let Some(branch) = branches.get(selected) {
        select_body(branch, next, selection, lines);
    }
    selected
}

/// 选择 `#SWITCH` 块。
fn select_switch(
    node: &SyntaxNode,
    next: &mut impl FnMut(u64) -> u64,
    selection: &mut BranchSelection,
    lines: &mut Vec<Line>,
) {
    let value = block_value(node, next);
    let cases: Vec<_> = node
        .children()
        .filter(|child| child.kind() == SyntaxKind::SwitchCase)
        .collect();
    let selected = cases
        .iter()
        .position(|case| case_matches(case, value))
        .unwrap_or(cases.len());

    for case in cases.iter().skip(selected) {
        select_body(case, next, selection, lines);
        if has_skip(case) {
            break;
        }
    }

    for child in node.children() {
        if !matches!(
            child.kind(),
            SyntaxKind::SwitchCase | SyntaxKind::SwitchDirective | SyntaxKind::EndSwDirective
        ) {
            select_node(&child, next, selection, lines);
        }
    }

    selection.decisions.push(BlockDecision {
        value,
        selected_index: selected,
    });
}

/// 选择分支/case 的内容，跳过指令行。
fn select_body(
    node: &SyntaxNode,
    next: &mut impl FnMut(u64) -> u64,
    selection: &mut BranchSelection,
    lines: &mut Vec<Line>,
) {
    for child in node.children() {
        if !child.kind().is_directive() {
            select_node(&child, next, selection, lines);
        }
    }
}

/// 读取块指令的随机值。
fn block_value(node: &SyntaxNode, next: &mut impl FnMut(u64) -> u64) -> u64 {
    let directive = node
        .children()
        .find(|child| child.kind().is_directive())
        .and_then(Line::cast);
    let Some(directive_line) = directive else {
        return 0;
    };
    let value = directive_line
        .value()
        .and_then(|token| token.text().parse::<u64>().ok())
        .unwrap_or(0);
    if directive_line.command_name().is_some_and(|token| {
        token.text().eq_ignore_ascii_case("SETRANDOM")
            || token.text().eq_ignore_ascii_case("SETSWITCH")
    }) {
        value
    } else if value == 0 {
        0
    } else {
        next(value)
    }
}

/// 判断分支是否匹配随机值。
fn branch_matches(branch: &SyntaxNode, value: u64) -> bool {
    let Some(line) = Line::cast(branch.children().next().unwrap_or_else(|| branch.clone())) else {
        return false;
    };
    let Some(command) = line.command_name() else {
        return false;
    };
    if command.text().eq_ignore_ascii_case("ELSE") {
        return true;
    }
    line.value()
        .and_then(|token| token.text().parse::<u64>().ok())
        .is_some_and(|branch_value| branch_value == value)
}

/// 判断 case 是否匹配随机值。
fn case_matches(case: &SyntaxNode, value: u64) -> bool {
    let Some(line) = Line::cast(case.children().next().unwrap_or_else(|| case.clone())) else {
        return false;
    };
    let Some(command) = line.command_name() else {
        return false;
    };
    if command.text().eq_ignore_ascii_case("DEF") {
        return true;
    }
    line.value()
        .and_then(|token| token.text().parse::<u64>().ok())
        .is_some_and(|case_value| case_value == value)
}

/// 判断 case 是否带 `#SKIP`。
fn has_skip(case: &SyntaxNode) -> bool {
    case.children()
        .any(|child| child.kind() == SyntaxKind::SkipDirective)
}
