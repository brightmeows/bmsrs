//! Control-flow branch selection queries.
//!
//! Branch decisions are driven either by an RNG ([`SourceFile::select_lines`])
//! or by a recorded [`BranchSelection`] ([`SourceFile::select_lines_with`]).
//! A recorded replay is strict: when the file presents a different number of
//! blocks, different block kinds, or different declared bounds than the
//! recording, the replay fails with [`BranchMismatch`] naming the first
//! divergent block.

use std::fmt;

use crate::{SourceFile, SyntaxKind, SyntaxLine, SyntaxNode};

/// Which kind of control-flow block a decision belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// A `#RANDOM` block.
    Random,
    /// A `#SWITCH` block.
    Switch,
}

impl BlockKind {
    /// Return the directive label of the block kind.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Random => "RANDOM",
            Self::Switch => "SWITCH",
        }
    }
}

impl fmt::Display for BlockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Decision for a single control-flow block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDecision {
    /// Kind of the block the decision belongs to.
    kind: BlockKind,
    /// Declared bound; `None` for set directives, `Some(0)` for zero or
    /// unparseable bounds.
    max: Option<u64>,
    /// Random value used for branch matching.
    value: u64,
    /// Index of the first matched branch/case; the branch count when nothing
    /// matched.
    selected_index: usize,
}

impl BlockDecision {
    /// Return the kind of the block.
    #[must_use]
    pub const fn kind(self) -> BlockKind {
        self.kind
    }

    /// Return the declared bound of the block: `None` for `#SETRANDOM` /
    /// `#SETSWITCH` (the directive carries a fixed value, not a bound), and
    /// `Some(bound)` for `#RANDOM` / `#SWITCH` (`Some(0)` when the directive
    /// value is zero or unparseable).
    #[must_use]
    pub const fn max(self) -> Option<u64> {
        self.max
    }

    /// Return the random value used.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }

    /// Return the index of the selected branch/case; the branch count when
    /// nothing matched.
    #[must_use]
    pub const fn selected_index(self) -> usize {
        self.selected_index
    }
}

/// Why a recorded replay diverged from the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchMismatch {
    /// Zero-based position of the first divergent block.
    block_index: usize,
    /// Block the file presents at that position.
    expected: Option<(BlockKind, Option<u64>)>,
    /// Block the recording presents at that position.
    recorded: Option<(BlockKind, Option<u64>)>,
}

impl BranchMismatch {
    /// The zero-based position of the first divergent block.
    #[must_use]
    pub const fn block_index(&self) -> usize {
        self.block_index
    }

    /// The block the file presents at that position: kind plus declared max.
    /// `None` when the file has no block there and the recording does.
    #[must_use]
    pub const fn expected(&self) -> Option<(BlockKind, Option<u64>)> {
        self.expected
    }

    /// The block the recording presents at that position. `None` when the
    /// recording ended before the file did.
    #[must_use]
    pub const fn recorded(&self) -> Option<(BlockKind, Option<u64>)> {
        self.recorded
    }
}

/// Describe a block kind plus bound for error messages.
fn describe_block(kind: BlockKind, max: Option<u64>) -> String {
    max.map_or_else(
        || format!("{} with a fixed value", kind.label()),
        |bound| format!("{} with max {bound}", kind.label()),
    )
}

impl fmt::Display for BranchMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "branch selection mismatch at block {}", self.block_index)?;
        match (self.expected, self.recorded) {
            (Some((kind, max)), Some((recorded_kind, recorded_max))) => write!(
                f,
                ": the file has {}, the recording has {}",
                describe_block(kind, max),
                describe_block(recorded_kind, recorded_max)
            ),
            (Some((kind, max)), None) => {
                write!(
                    f,
                    ": the file has {}, but the recording ended",
                    describe_block(kind, max)
                )
            }
            (None, Some((kind, max))) => write!(
                f,
                ": the recording has {}, but the file has no block there",
                describe_block(kind, max)
            ),
            (None, None) => f.write_str(": no decision available"),
        }
    }
}

impl std::error::Error for BranchMismatch {}

/// Decisions for all control-flow blocks, in traversal order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchSelection {
    /// Decisions in traversal order.
    decisions: Vec<BlockDecision>,
}

impl BranchSelection {
    /// Return the decision list.
    #[must_use]
    pub fn decisions(&self) -> &[BlockDecision] {
        &self.decisions
    }
}

/// Where a block's random value comes from.
enum Source<'a> {
    /// Draw values from an RNG; each consuming block calls it with its
    /// declared bound.
    Rng(Box<dyn FnMut(u64) -> u64 + 'a>),
    /// Draw values from a recorded selection; each block is validated against
    /// the recorded kind and bound.
    Recorded(std::slice::Iter<'a, BlockDecision>),
}

/// Traversal state shared by the selection entry points.
struct Walker<'a> {
    /// Value source for the current pass.
    source: Source<'a>,
    /// Zero-based position of the next control-flow block.
    block_index: usize,
    /// First replay mismatch, when replaying.
    error: Option<BranchMismatch>,
    /// Decisions recorded during the current pass.
    selection: BranchSelection,
}

impl Walker<'_> {
    /// Resolve the value of one block directive and record the decision
    /// metadata. Returns `None` when a recorded replay diverged; the mismatch
    /// is stored on the walker.
    fn block_value(&mut self, node: &SyntaxNode, kind: BlockKind) -> Option<(u64, Option<u64>)> {
        let index = self.block_index;
        self.block_index += 1;

        let directive = node
            .children()
            .find(|child| child.kind().is_directive())
            .and_then(SyntaxLine::cast);
        let Some(directive_line) = directive else {
            return Some((0, Some(0)));
        };
        let raw = directive_line
            .value()
            .and_then(|token| token.text().parse::<u64>().ok())
            .unwrap_or(0);
        let is_set = directive_line.command_name().is_some_and(|token| {
            token.text().eq_ignore_ascii_case("SETRANDOM")
                || token.text().eq_ignore_ascii_case("SETSWITCH")
        });
        let declared_max = if is_set { None } else { Some(raw) };

        match &mut self.source {
            Source::Rng(next) => {
                let value = match (is_set, raw) {
                    (true, fixed) => fixed,
                    (false, 0) => 0,
                    (false, bound) => next(bound),
                };
                Some((value, declared_max))
            }
            Source::Recorded(iter) => {
                let Some(recorded) = iter.next() else {
                    self.error = Some(BranchMismatch {
                        block_index: index,
                        expected: Some((kind, declared_max)),
                        recorded: None,
                    });
                    return None;
                };
                if recorded.kind() != kind || recorded.max() != declared_max {
                    self.error = Some(BranchMismatch {
                        block_index: index,
                        expected: Some((kind, declared_max)),
                        recorded: Some((recorded.kind(), recorded.max())),
                    });
                    return None;
                }
                // A set directive takes its value from the current file, so
                // directive-value edits stay visible to replays; genuine
                // random blocks replay the recorded value.
                let value = if is_set { raw } else { recorded.value() };
                Some((value, declared_max))
            }
        }
    }

    /// Select every child of the node, depth-first.
    fn select_children(&mut self, parent: &SyntaxNode, lines: &mut Vec<SyntaxLine>) {
        for child in parent.children() {
            self.select_node(&child, lines);
        }
    }

    /// Dispatch one node to its block handler or collect it as a line.
    fn select_node(&mut self, node: &SyntaxNode, lines: &mut Vec<SyntaxLine>) {
        match node.kind() {
            SyntaxKind::RandomBlock => self.select_random(node, lines),
            SyntaxKind::SwitchBlock => self.select_switch(node, lines),
            SyntaxKind::RandomChain => {
                self.select_chain(node, 0, lines);
            }
            SyntaxKind::RandomBranch | SyntaxKind::SwitchCase => {
                self.select_body(node, lines);
            }
            kind if kind.is_directive() => {}
            _ => {
                if let Some(line) = SyntaxLine::cast(node.clone()) {
                    lines.push(line);
                } else {
                    self.select_children(node, lines);
                }
            }
        }
    }

    /// Select one `#RANDOM` block and record its decision.
    fn select_random(&mut self, node: &SyntaxNode, lines: &mut Vec<SyntaxLine>) {
        if self.error.is_some() {
            return;
        }
        let Some((value, max)) = self.block_value(node, BlockKind::Random) else {
            return;
        };
        let mut first_selected = 0usize;
        let mut chain_index = 0usize;

        for child in node.children() {
            match child.kind() {
                SyntaxKind::RandomChain => {
                    let selected = self.select_chain(&child, value, lines);
                    if chain_index == 0 {
                        first_selected = selected;
                    }
                    chain_index = chain_index.saturating_add(1);
                }
                SyntaxKind::RandomDirective | SyntaxKind::EndRandomDirective => {}
                _ => self.select_node(&child, lines),
            }
        }

        self.selection.decisions.push(BlockDecision {
            kind: BlockKind::Random,
            max,
            value,
            selected_index: first_selected,
        });
    }

    /// Select one mutually exclusive `#IF` chain.
    fn select_chain(
        &mut self,
        chain: &SyntaxNode,
        value: u64,
        lines: &mut Vec<SyntaxLine>,
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
            self.select_body(branch, lines);
        }
        selected
    }

    /// Select one `#SWITCH` block and record its decision.
    fn select_switch(&mut self, node: &SyntaxNode, lines: &mut Vec<SyntaxLine>) {
        if self.error.is_some() {
            return;
        }
        let Some((value, max)) = self.block_value(node, BlockKind::Switch) else {
            return;
        };
        let cases: Vec<_> = node
            .children()
            .filter(|child| child.kind() == SyntaxKind::SwitchCase)
            .collect();
        let selected = cases
            .iter()
            .position(|case| case_matches(case, value))
            .unwrap_or(cases.len());

        for case in cases.iter().skip(selected) {
            self.select_body(case, lines);
            if has_skip(case) {
                break;
            }
        }

        for child in node.children() {
            if !matches!(
                child.kind(),
                SyntaxKind::SwitchCase | SyntaxKind::SwitchDirective | SyntaxKind::EndSwDirective
            ) {
                self.select_node(&child, lines);
            }
        }

        self.selection.decisions.push(BlockDecision {
            kind: BlockKind::Switch,
            max,
            value,
            selected_index: selected,
        });
    }

    /// Select the body of a branch/case, skipping directive lines.
    fn select_body(&mut self, node: &SyntaxNode, lines: &mut Vec<SyntaxLine>) {
        for child in node.children() {
            if !child.kind().is_directive() {
                self.select_node(&child, lines);
            }
        }
    }
}

impl SourceFile {
    /// Select branches using the random values supplied by `next`, and return
    /// the selected lines together with the recorded decisions.
    ///
    /// `next(max)` should return a value in `[1, max]`; `#SETRANDOM` /
    /// `#SETSWITCH` do not call it.
    #[must_use]
    pub fn select_lines(&self, next: impl FnMut(u64) -> u64) -> (Vec<SyntaxLine>, BranchSelection) {
        let mut walker = Walker {
            source: Source::Rng(Box::new(next)),
            block_index: 0,
            error: None,
            selection: BranchSelection::default(),
        };
        let mut lines = Vec::new();
        walker.select_children(self.syntax(), &mut lines);
        (lines, walker.selection)
    }

    /// Select branches using the recorded decisions, and return the selected
    /// lines together with the replayed decisions.
    ///
    /// The replay is strict: the file must present the same number of blocks
    /// with the same kinds and declared bounds as the recording, otherwise
    /// [`BranchMismatch`] names the first divergent block. `#SETRANDOM` /
    /// `#SETSWITCH` values are taken from the current file, not from the
    /// recording, so directive-value edits stay visible.
    ///
    /// # Errors
    ///
    /// Returns [`BranchMismatch`] naming the first divergent block when the
    /// recording does not match the file's control-flow structure.
    pub fn select_lines_with(
        &self,
        selection: &BranchSelection,
    ) -> Result<(Vec<SyntaxLine>, BranchSelection), BranchMismatch> {
        let mut walker = Walker {
            source: Source::Recorded(selection.decisions().iter()),
            block_index: 0,
            error: None,
            selection: BranchSelection::default(),
        };
        let mut lines = Vec::new();
        walker.select_children(self.syntax(), &mut lines);
        match walker.error {
            Some(error) => Err(error),
            None => Ok((lines, walker.selection)),
        }
    }
}

/// Return whether a branch matches the random value.
fn branch_matches(branch: &SyntaxNode, value: u64) -> bool {
    let Some(line) = SyntaxLine::cast(branch.children().next().unwrap_or_else(|| branch.clone()))
    else {
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
    let Some(line) = SyntaxLine::cast(case.children().next().unwrap_or_else(|| case.clone()))
    else {
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
