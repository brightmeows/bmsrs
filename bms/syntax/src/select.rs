//! Control-flow branch selection queries.

use crate::{Line, SourceFile, SyntaxKind, SyntaxNode};

/// Decision for a single control-flow block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDecision {
    /// Random value used.
    value: u64,
    /// Index of the selected branch/case; the branch count when nothing
    /// matched.
    selected_index: usize,
}

impl BlockDecision {
    /// Return the random value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }

    /// Return the selected branch/case index.
    #[must_use]
    pub const fn selected_index(self) -> usize {
        self.selected_index
    }
}

/// Decisions for all control-flow blocks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchSelection {
    /// Decisions recorded in traversal order.
    decisions: Vec<BlockDecision>,
}

impl BranchSelection {
    /// Return the decision list.
    #[must_use]
    pub fn decisions(&self) -> &[BlockDecision] {
        &self.decisions
    }
}

impl SourceFile {
    /// Select branches using the random values supplied by `next`, and return
    /// the selected lines.
    ///
    /// `next(max)` should return a value in `[1, max]`; `#SETRANDOM` /
    /// `#SETSWITCH` do not call it.
    #[must_use]
    pub fn select_lines(&self, mut next: impl FnMut(u64) -> u64) -> (Vec<Line>, BranchSelection) {
        let mut selection = BranchSelection::default();
        let mut lines = Vec::new();
        select_children(self.syntax(), &mut next, &mut selection, &mut lines);
        (lines, selection)
    }
}

/// Select every item under a parent node.
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

/// Select a single node.
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

/// Select a `#RANDOM` block.
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

/// Select one mutually exclusive chain.
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

/// Select a `#SWITCH` block.
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

/// Select the body of a branch/case, skipping directive lines.
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

/// Read the random value of a block directive.
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

/// Return whether a branch matches the random value.
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

/// Return whether a case matches the random value.
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

/// Return whether a case has a `#SKIP`.
fn has_skip(case: &SyntaxNode) -> bool {
    case.children()
        .any(|child| child.kind() == SyntaxKind::SkipDirective)
}
