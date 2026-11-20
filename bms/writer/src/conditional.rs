//! `KaleidBeat` conditional-block serialization.

use bmsrs_bms_model::ConditionalBlock;

/// Serialize every `#CONDITIONAL` block in source order.
///
/// The first conditional branch is written as `#WHEN`, later ones as
/// `#ELSEWHEN`, and the branch without a condition as `#DEFAULT`.
#[must_use]
pub fn write_blocks(blocks: &[ConditionalBlock]) -> Vec<String> {
    let mut out = Vec::new();
    for block in blocks {
        out.push(format!("#CONDITIONAL {}", block.position));
        let mut seen_condition = false;
        for branch in &block.branches {
            match branch.condition {
                Some(condition) if seen_condition => {
                    out.push(format!("#ELSEWHEN {condition}"));
                }
                Some(condition) => {
                    out.push(format!("#WHEN {condition}"));
                    seen_condition = true;
                }
                None => out.push("#DEFAULT".to_owned()),
            }
            out.extend(
                branch
                    .messages
                    .iter()
                    .map(bmsrs_bms_values::BmsMessage::to_line_text),
            );
        }
        out.push("#ENDCONDITIONAL".to_owned());
    }
    out
}
