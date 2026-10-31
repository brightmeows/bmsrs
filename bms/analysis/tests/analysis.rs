//! Analysis-layer tests.

use bmsrs_bms_analysis::{Analysis, SymbolKind};

#[test]
fn wav_definition_and_reference_are_indexed() {
    let analysis = Analysis::from_text("#WAV01 kick.wav\n#00111:0100\n");
    let definitions = analysis
        .index()
        .definitions_for(SymbolKind::Wav, "01")
        .count();
    let references = analysis
        .index()
        .references_for(SymbolKind::Wav, "01")
        .count();
    assert_eq!((definitions, references), (1, 1));
}

#[test]
fn missing_wav_reference_is_reported() {
    let analysis = Analysis::from_text("#00111:0200\n");
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("undefined WAV 02"))
    );
}

#[test]
fn duplicate_definition_is_reported() {
    let analysis = Analysis::from_text("#WAV01 a.wav\n#WAV01 b.wav\n");
    assert!(analysis.diagnostics().iter().any(|diagnostic| {
        diagnostic
            .message()
            .contains("duplicate definition of WAV 01")
    }));
}

#[test]
fn bga_reference_accepts_bmp_fallback() {
    let analysis = Analysis::from_text("#BMP01 bg.png\n#00104:0100\n");
    assert!(
        !analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("undefined BGA 01"))
    );
}

#[test]
fn unclosed_random_is_reported() {
    let analysis = Analysis::from_text("#RANDOM 1\n#IF 1\n#TITLE A\n");
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("unclosed #RANDOM"))
    );
}

#[test]
fn invalid_header_value_is_reported() {
    let analysis = Analysis::from_text("#BPM abc\n");
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("invalid float"))
    );
}

#[test]
fn header_doc_matches_command_with_id() {
    let found = bmsrs_bms_analysis::header_doc("#WAV01");
    assert_eq!(found.map(|doc| doc.command), Some("WAV"));
}

#[test]
fn header_doc_matches_at_bga() {
    let found = bmsrs_bms_analysis::header_doc("#@BGA01");
    assert_eq!(found.map(|doc| doc.command), Some("@BGA"));
}

#[test]
fn channel_doc_matches_note_group() {
    let found = bmsrs_bms_analysis::channel_doc("11");
    assert_eq!(found.map(|doc| doc.title), Some("1P visible notes"));
}

#[test]
fn noncanonical_message_line_indexes_references() {
    let analysis = Analysis::from_text("#1:0101\n");
    assert_eq!(
        analysis
            .index()
            .references_for(SymbolKind::Wav, "01")
            .count(),
        2
    );
}
