//! 行导向控制流视图测试。

use bms_control_flow::{BranchRng, ControlFlowError, FlowBlock, FlowDoc, FlowNode};
use bms_document::{Document, LineRef};

/// 返回固定值的 RNG。
struct Fixed(u64);

impl BranchRng for Fixed {
    fn gen_range(&mut self, _max: u64) -> u64 {
        self.0
    }
}

/// 含 `#RANDOM` 两分支的测试谱面。
const SOURCE: &str = "\
#TITLE A
#RANDOM 2
#IF 1
#00111:0101
#ENDIF
#IF 2
#00111:0202
#ENDIF
#ENDRANDOM
";

fn flow() -> Result<FlowDoc<LineRef>, ControlFlowError> {
    Document::from_text(SOURCE).flow()
}

#[test]
fn flow_keeps_top_level_payload_and_block() {
    let document = flow().unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.len(), 2);
}

#[test]
fn flow_block_has_one_chain_per_if_group() {
    let document = flow().unwrap_or_else(|error| panic!("{error}"));
    let chains = match document.get(1) {
        Some(FlowNode::Block(FlowBlock::Random(random))) => Some(random.chains.len()),
        _ => None,
    };
    assert_eq!(chains, Some(2));
}

#[test]
fn selected_branch_renders_original_bytes() {
    let source = Document::from_text(SOURCE);
    let document = source.flow().unwrap_or_else(|error| panic!("{error}"));
    let (ranges, _selection) = document.select_payloads(&mut Fixed(1));
    assert_eq!(source.render_lines(&ranges), b"#TITLE A\n#00111:0101\n");
}

#[test]
fn second_branch_renders_original_bytes() {
    let source = Document::from_text(SOURCE);
    let document = source.flow().unwrap_or_else(|error| panic!("{error}"));
    let (ranges, _selection) = document.select_payloads(&mut Fixed(2));
    assert_eq!(source.render_lines(&ranges), b"#TITLE A\n#00111:0202\n");
}

#[test]
fn trivia_lines_stay_with_their_branch() {
    let source = Document::from_text(
        "#TITLE A\n// top\n#RANDOM 1\n#IF 1\n// inside\n#00111:0101\n#ENDIF\n#ENDRANDOM\n",
    );
    let document = source.flow().unwrap_or_else(|error| panic!("{error}"));
    let (ranges, _selection) = document.select_payloads(&mut Fixed(1));
    assert_eq!(
        source.render_lines(&ranges),
        b"#TITLE A\n// top\n// inside\n#00111:0101\n"
    );
}

#[test]
fn line_ref_len_counts_inclusive_endpoints() {
    let start = std::num::NonZeroUsize::MIN;
    let end = std::num::NonZeroUsize::new(2).unwrap_or(std::num::NonZeroUsize::MIN);
    assert_eq!(LineRef::new(start, end).line_count(), 2);
}
