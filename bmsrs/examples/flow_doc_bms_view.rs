//! 示例：从 `FlowDoc<TokenPayload<C>>` 派生出 `FlowDoc<Bms>` 视图。
//!
//! `bms-control-flow` 并不知道 `Bms` 的存在——这正是 payload 泛型设计的
//! 核心目的。本示例展示下游使用模式：先构建 token 级控制流树，再用
//! [`FlowDoc::map_payload`] 把每个 payload 区间规约为已解析的 `Bms`，
//! 同时保持 Random/Switch 骨架不变。这是谱面编辑器查看每个分支变体
//! 已解析内容时所需的数据形态。
//!
//! 运行方式：
//!
//! ```sh
//! cargo run --example flow_doc_bms_view
//! ```

// 示例的职责就是向 stdout 输出；工作区内禁用
// `println!`（改用 `tracing`）的规则在此不适用。
#![expect(clippy::print_stdout, reason = "example prints to stdout by design")]

use bmsrs::bms::control_flow::{FlowBlock, FlowDoc, FlowNode, TokenPayload};
use bmsrs::bms::parser::Bms;
use bmsrs::bms::tokenizer::BmsTokenizer;

/// 一份极小谱面：含一个顶层区间和一个 `#RANDOM` 块，
/// 后者分裂为两条分支，各带自己的通道消息。
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
    // 1. 将 BMS 文本分词为 `(line, token)` 流。
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, String>(SOURCE)
        .into_iter()
        .filter_map(|(line, res)| res.ok().map(|token| (line, token)))
        .collect();

    // 2. 构建 token 级控制流树——可编辑的事实来源。
    //    连续的非控制流 token 打包成 payload 区间；
    //    `#RANDOM`/`#IF` 变为结构化分支。
    let token_tree = FlowDoc::from_tokens(tokens)?;

    // 3. 派生 `FlowDoc<Bms>`：每个 payload 区间通过
    //    `Bms::from_flat_tokens` 规约为 `Bms`。控制流骨架原样保留，
    //    只是叶子 payload 类型发生了变化。
    let bms_tree: FlowDoc<Bms> = token_tree.map_payload(
        |TokenPayload {
             tokens: payload_tokens,
         }| { Bms::from_flat_tokens(payload_tokens.into_iter().map(|(_, token)| token)) },
    );

    // 4. 遍历结果树：每个区间打印其已解析 `Bms` 摘要，
    //    按原始控制流结构嵌套输出。
    println!("FlowDoc<Bms> — skeleton with per-span parsed Bms:");
    walk(&bms_tree, 0);
    Ok(())
}

/// 递归打印 `FlowDoc<Bms>` 节点列表，并附每区间的摘要。
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
                println!("{indent}▸ #RANDOM ({} chains)", r.chains.len());
                for chain in &r.chains {
                    println!("{indent}  chain ({} branches)", chain.branches.len());
                    for branch in &chain.branches {
                        println!("{indent}    branch {:?}", branch.kind);
                        walk(&branch.body, depth + 3);
                    }
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
