//! Consistency between the resource definition table and `bmsrs-bms-values` headers.
//!
//! `Analysis::definition_kind` maintains a command-prefix table. This test emits
//! one line per resource header and asserts that the symbol index recognizes the
//! corresponding kind and ID. If `bmsrs-bms-values` adds, removes, or renames a
//! header, this test fails first.

use bmsrs_bms_analysis::{Analysis, SymbolKind};

const DEFINITIONS: &str = "\
#WAV01 a.wav
#WAV02 b.wav
#EXWAV03 extra.ogg
#BMP01 bg.png
#EXBMP02 255,0,128,64 overlay.png
#BGA03 02 0 0 100 100 10 20
#@BGA04 03 5 10 200 150 0 0
#SWBGA05 30:60:1:0:255,0,0,128 pattern.bmp
#ARGB06 128,255,0,64
#BPM07 180.0
#EXBPM08 200.0
#STOP09 192
#SCROLL0A 1.0
#SPEED0B 1.0
#EXRANK0C 5
#CHANGEOPTION0D 774:HIDDEN_STEALTH
#SEEK0E 1.5
#TEXT0F \"hello\"
#SONG0G \"hello\"
";

#[test]
fn all_resource_headers_are_indexed() {
    let analysis = Analysis::from_text(DEFINITIONS);
    let expected = [
        (SymbolKind::Wav, "01"),
        (SymbolKind::Wav, "02"),
        (SymbolKind::Wav, "03"),
        (SymbolKind::Bmp, "01"),
        (SymbolKind::Bmp, "02"),
        (SymbolKind::Bga, "03"),
        (SymbolKind::Bga, "04"),
        (SymbolKind::Bga, "05"),
        (SymbolKind::Bga, "06"),
        (SymbolKind::Bpm, "07"),
        (SymbolKind::Bpm, "08"),
        (SymbolKind::Stop, "09"),
        (SymbolKind::Scroll, "0A"),
        (SymbolKind::Speed, "0B"),
        (SymbolKind::ExRank, "0C"),
        (SymbolKind::ChangeOption, "0D"),
        (SymbolKind::Seek, "0E"),
        (SymbolKind::Text, "0F"),
        (SymbolKind::Text, "0G"),
    ];
    for (kind, id) in expected {
        assert_eq!(
            analysis.index().definitions_for(kind, id).count(),
            1,
            "missing definition for {kind:?} {id}"
        );
    }
}
