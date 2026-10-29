//! Error-tolerant parsing: assemble line tokens into a rowan CST.

use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

use crate::ast::SourceFile;
use crate::lexer::{LineData, LineKind, lex};
use crate::{ParseOptions, SyntaxKind};

/// Diagnostic category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// Syntax error.
    Syntax,
    /// Control-flow structure error.
    ControlFlow,
}

/// A parse diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Text range.
    range: TextRange,
    /// Diagnostic message.
    message: String,
    /// Diagnostic category.
    kind: DiagnosticKind,
}

impl Diagnostic {
    /// Construct a diagnostic.
    fn new(range: TextRange, message: impl Into<String>, kind: DiagnosticKind) -> Self {
        Self {
            range,
            message: message.into(),
            kind,
        }
    }

    /// Return the text range.
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    /// Return the diagnostic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the diagnostic category.
    #[must_use]
    pub const fn kind(&self) -> DiagnosticKind {
        self.kind
    }
}

/// The result of one parse.
#[derive(Debug, Clone)]
pub struct Parse {
    /// Green tree.
    green: GreenNode,
    /// Diagnostic list.
    errors: Vec<Diagnostic>,
}

impl Parse {
    /// Return the document root node.
    #[must_use]
    pub fn source_file(&self) -> SourceFile {
        SourceFile(self.syntax())
    }

    /// Return a fresh red root node.
    #[must_use]
    pub fn syntax(&self) -> crate::SyntaxNode {
        crate::SyntaxNode::new_root(self.green.clone())
    }

    /// Return the diagnostic list.
    #[must_use]
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }
}

/// A CST item.
enum Item<'a> {
    /// Plain line.
    Line(LineData<'a>),
    /// `#RANDOM` block.
    Random(RandomBlock<'a>),
    /// `#SWITCH` block.
    Switch(SwitchBlock<'a>),
}

/// A `#RANDOM` block.
struct RandomBlock<'a> {
    /// Block opening directive.
    directive: LineData<'a>,
    /// Items outside the directive and branches.
    body: Vec<Item<'a>>,
    /// Mutually exclusive chains.
    chains: Vec<RandomChain<'a>>,
    /// `#ENDRANDOM`.
    end: Option<LineData<'a>>,
}

/// A `#IF` mutually exclusive chain.
struct RandomChain<'a> {
    /// Branch list.
    branches: Vec<RandomBranch<'a>>,
    /// `#ENDIF`.
    end: Option<LineData<'a>>,
}

/// A single branch in a mutually exclusive chain.
struct RandomBranch<'a> {
    /// Branch opening directive.
    directive: LineData<'a>,
    /// Branch body.
    body: Vec<Item<'a>>,
}

/// A `#SWITCH` block.
struct SwitchBlock<'a> {
    /// Block opening directive.
    directive: LineData<'a>,
    /// Items outside the cases.
    body: Vec<Item<'a>>,
    /// Case list.
    cases: Vec<SwitchCase<'a>>,
    /// `#ENDSW`.
    end: Option<LineData<'a>>,
}

/// A single `#SWITCH` case.
struct SwitchCase<'a> {
    /// Case opening directive.
    directive: LineData<'a>,
    /// Case body.
    body: Vec<Item<'a>>,
    /// `#SKIP`.
    skip: Option<LineData<'a>>,
}

/// Parse text and build the CST.
pub fn parse_with(text: &str, options: &ParseOptions) -> Parse {
    let lines = lex(text, options);
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

/// Collect error lines from the lexer.
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

/// Parse a range of lines.
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

/// Parse one item.
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

/// Build an empty line; only used as an out-of-bounds fallback.
const fn empty_line<'a>() -> LineData<'a> {
    LineData {
        kind: LineKind::Blank,
        tokens: Vec::new(),
        start: 0,
        end: 0,
    }
}

/// Parse one `#RANDOM` block.
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

/// Parse the inside of a `#RANDOM` block.
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

/// Close the current mutually exclusive chain.
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

/// Parse one `#SWITCH` block.
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

/// Parse the inside of a `#SWITCH` block.
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

/// Find the matching block end line.
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

/// Append an item to the current branch or block body.
fn push_item<'a>(body: &mut Vec<Item<'a>>, branch: &mut Option<RandomBranch<'a>>, item: Item<'a>) {
    if let Some(current) = branch {
        current.body.push(item);
    } else {
        body.push(item);
    }
}

/// Append an error line.
fn push_error<'a>(
    body: &mut Vec<Item<'a>>,
    branch: &mut Option<RandomBranch<'a>>,
    line: &LineData<'a>,
    message: &str,
    errors: &mut Vec<Diagnostic>,
) {
    push_item(body, branch, Item::Line(error_line(line, message, errors)));
}

/// Mark a line as an error and record a diagnostic.
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

/// Build the CST iteratively.
fn build_items(builder: &mut GreenNodeBuilder<'_>, items: &[Item<'_>]) {
    for item in items {
        match item {
            Item::Line(line) => build_line(builder, line),
            Item::Random(block) => build_random(builder, block),
            Item::Switch(block) => build_switch(builder, block),
        }
    }
}

/// Build a plain line node.
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

/// Build a node from a line and its tokens.
fn build_node(builder: &mut GreenNodeBuilder<'_>, line: &LineData<'_>, kind: SyntaxKind) {
    builder.start_node(kind.raw());
    for token in &line.tokens {
        builder.token(token.kind.raw(), token.text);
    }
    builder.finish_node();
}

/// Build a `#RANDOM` block.
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

/// Build a `#SWITCH` block.
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

/// Return the node kind for a branch directive.
fn branch_kind(line: &LineData<'_>) -> SyntaxKind {
    if is_command(line, "IF") {
        SyntaxKind::IfDirective
    } else if is_command(line, "ELSEIF") {
        SyntaxKind::ElseIfDirective
    } else {
        SyntaxKind::ElseDirective
    }
}

/// Return the node kind for a case directive.
fn case_kind(line: &LineData<'_>) -> SyntaxKind {
    if is_command(line, "DEF") {
        SyntaxKind::DefDirective
    } else {
        SyntaxKind::CaseDirective
    }
}

/// Return the line's text range.
fn line_range(line: &LineData<'_>) -> TextRange {
    TextRange::new(text_size(line.start), text_size(line.end))
}

/// Convert a byte offset to `TextSize`.
fn text_size(offset: usize) -> TextSize {
    TextSize::from(u32::try_from(offset).unwrap_or(u32::MAX))
}

/// Whether this is `#RANDOM` or `#SETRANDOM`.
fn is_random_open(line: &LineData<'_>) -> bool {
    is_command(line, "RANDOM") || is_command(line, "SETRANDOM") || is_command(line, "RONDAM")
}

/// Whether this is `#ENDRANDOM`.
fn is_end_random(line: &LineData<'_>) -> bool {
    is_command(line, "ENDRANDOM")
}

/// Whether this is `#SWITCH` or `#SETSWITCH`.
fn is_switch_open(line: &LineData<'_>) -> bool {
    is_command(line, "SWITCH") || is_command(line, "SETSWITCH")
}

/// Whether this is `#ENDSW`.
fn is_end_switch(line: &LineData<'_>) -> bool {
    is_command(line, "ENDSW") || is_command(line, "ENDSWITCH")
}

/// Whether this is any control-flow directive.
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

/// Return whether the command name matches.
fn is_command(line: &LineData<'_>, command: &str) -> bool {
    line.command_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(command))
}
