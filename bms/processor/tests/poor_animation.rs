//! Poor-animation channel (`#xxx06`) tests: jbms-parser miss-animation
//! semantics for multi-object lines.

use bmsrs_bms_parser::BmsParser;
use bmsrs_bms_processor::layout::Bme;
use bmsrs_bms_processor::{BmsProcessor, ProcessWarningKind};
use bmsrs_bms_syntax::SourceFile;
use bmsrs_chart::{BgaLayer, BgaResourceContent, BgaResourceId, EventKind};

type Outcome =
    bmsrs_pipeline::Outcome<bmsrs_bms_processor::Processed, bmsrs_bms_processor::ProcessWarning>;

/// Parse and process the fixture with the default beat layout.
#[expect(
    clippy::unwrap_used,
    reason = "static fixture; a failure is a test bug"
)]
fn process(text: &str) -> Outcome {
    let parse = SourceFile::parse(text);
    let bms = BmsParser::parse(&parse.source_file(), |_| 1).bms;
    BmsProcessor::process::<Bme>(&bms).unwrap()
}

fn poor_events(processed: &Outcome) -> Vec<(u64, Option<u32>)> {
    processed
        .value
        .chart
        .data()
        .events()
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::Bga {
                layer: BgaLayer::Poor,
                resource_id,
            } => Some((event.tick(), resource_id.map(BgaResourceId::get))),
            _ => None,
        })
        .collect()
}

#[test]
fn multi_object_poor_line_becomes_miss_animation() {
    let processed = process(
        "#BPM 120\n\
         #BMP01 a.png\n\
         #BMP02 b.png\n\
         #BMP03 c.png\n\
         #00006:010203\n",
    );

    assert_eq!(processed.value.chart.chart_info().bga_resources.len(), 4);
    let BgaResourceContent::Sequence { frames } =
        &processed.value.chart.chart_info().bga_resources[3]
    else {
        panic!("the animated line becomes a sequence resource");
    };
    assert_eq!(
        frames
            .iter()
            .map(|frame| frame.resource.map(BgaResourceId::get))
            .collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2)]
    );
    // jbms-parser spreads 500 ms uniformly across the frames.
    let total: std::time::Duration = frames.iter().map(|frame| frame.duration).sum();
    assert!(total.as_millis() >= 499 && total.as_millis() <= 501);

    // Exactly one poor event at the measure line references the sequence.
    assert_eq!(poor_events(&processed), vec![(0, Some(3))]);
}

#[test]
fn single_object_poor_line_stays_a_timed_switch() {
    let processed = process(
        "#BPM 120\n\
         #BMP01 a.png\n\
         #00106:0001\n",
    );

    assert_eq!(processed.value.chart.chart_info().bga_resources.len(), 1);
    assert!(
        processed
            .value
            .chart
            .chart_info()
            .bga_resources
            .iter()
            .all(|resource| matches!(resource, BgaResourceContent::File { .. }))
    );
    assert_eq!(poor_events(&processed), vec![(1440, Some(0))]);
}

#[test]
fn undefined_object_becomes_blank_frame_and_warns() {
    let processed = process(
        "#BPM 120\n\
         #BMP01 a.png\n\
         #00006:0701\n",
    );

    let BgaResourceContent::Sequence { frames } =
        &processed.value.chart.chart_info().bga_resources[1]
    else {
        panic!("the animated line becomes a sequence resource");
    };
    assert_eq!(frames[0].resource, None, "undefined 07 is a blank frame");
    assert_eq!(frames[1].resource, Some(BgaResourceId::new(0)));
    assert!(processed.warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::MissingBmpDefinition { key: 7, .. }
    )));
}
