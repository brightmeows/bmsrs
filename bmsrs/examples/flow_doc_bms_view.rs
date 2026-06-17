//! Example: derive a `FlowDoc<Bms>` view from a `FlowDoc<TokenPayload<C>>`.
//!
//! `bms-control-flow` does not know about `Bms` — that is the whole point of
//! the payload-generic design. This example shows the downstream pattern:
//! build the token-level control-flow tree, then use [`FlowDoc::map_payload`]
//! to reduce each payload span into a parsed `Bms`, keeping the Random/Switch
//! skeleton intact. This is the data shape a chart editor would use to
//! inspect every branch variant's parsed content.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example flow_doc_bms_view
//! ```

// An example's job is to print to stdout; the workspace-wide ban on
// `println!` (in favour of `tracing`) does not serve that purpose here.
#![expect(
    clippy::disallowed_macros,
    reason = "example prints to stdout by design"
)]

use bmsrs::bms::control_flow::{FlowBlock, FlowDoc, FlowNode, TokenPayload};
use bmsrs::bms::parser::Bms;
use bmsrs::bms::tokenizer::BmsTokenizer;

/// A tiny chart with a top-level span and a `#RANDOM` block splitting into
/// two branches, each with its own channel message.
const SOURCE: &str = "\
#TITLE Demo
#BPM 120
#WAV01 a.wav
#WAV02 b.wav
#RANDOM 2
#IF 1
#00101:0101
#ENDIF
#IF 2
#00101:0202
#ENDIF
#ENDRANDOM
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Tokenize the BMS text into a `(line, token)` stream.
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, String>(SOURCE)
        .into_iter()
        .filter_map(|(line, res)| res.ok().map(|token| (line, token)))
        .collect();

    // 2. Build the token-level control-flow tree — the editable source of
    //    truth. Consecutive non-control-flow tokens pack into payload spans;
    //    `#RANDOM`/`#IF` become structured branches.
    let token_tree = FlowDoc::from_tokens(tokens)?;

    // 3. Derive `FlowDoc<Bms>`: every payload span is reduced to a `Bms` via
    //    `Bms::from_flat_tokens`. The control-flow skeleton is preserved
    //    verbatim — only the leaf payload kind changes.
    let bms_tree: FlowDoc<Bms> = token_tree.map_payload(|TokenPayload { tokens }| {
        Bms::from_flat_tokens(tokens.into_iter().map(|(_, token)| token))
    });

    // 4. Walk the resulting tree: each span prints its parsed `Bms` summary,
    //    nested under the original control-flow structure.
    println!("FlowDoc<Bms> — skeleton with per-span parsed Bms:");
    walk(&bms_tree, 0);
    Ok(())
}

/// Recursively print a `FlowDoc<Bms>` node list with per-span summaries.
fn walk(nodes: &[FlowNode<Bms>], depth: usize) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        match node {
            FlowNode::Payload(bms) => {
                let title = bms.metadata.title.as_deref().unwrap_or("(untitled)");
                let events = bms.messages.bgm_events.len() + bms.messages.note_events.len();
                println!(
                    "{indent}• span: title={title:?}, bpm={:?}, events={events}",
                    bms.timing.bpm
                );
            }
            FlowNode::Block(FlowBlock::Random(r)) => {
                println!("{indent}▸ #RANDOM ({} branches)", r.branches.len());
                for branch in &r.branches {
                    println!("{indent}  branch {:?}", branch.kind);
                    walk(&branch.body, depth + 2);
                }
            }
            FlowNode::Block(FlowBlock::Switch(s)) => {
                println!("{indent}▸ #SWITCH ({} cases)", s.cases.len());
                for case in &s.cases {
                    println!("{indent}  case {:?}", case.kind);
                    walk(&case.body, depth + 2);
                }
            }
        }
    }
}
