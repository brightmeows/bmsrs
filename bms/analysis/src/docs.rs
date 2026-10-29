//! English descriptions for header commands and channels.
//!
//! This table is a manually maintained summary; the authoritative source is the
//! bms skill (`~/.agents/skills/bms/`) and the BMS command memo it points to.
//! Check the skill before changing a summary to avoid drifting from the
//! specification.

/// Description of a header command.
#[derive(Debug, Clone, Copy)]
pub struct HeaderDoc {
    /// Command prefix (without `#` / `%`).
    pub command: &'static str,
    /// Display title.
    pub title: &'static str,
    /// Brief description.
    pub summary: &'static str,
    /// Link to the original reference.
    pub link: &'static str,
}

/// Description of a channel.
#[derive(Debug, Clone, Copy)]
pub struct ChannelDoc {
    /// Channel number or channel group.
    pub code: &'static str,
    /// Display title.
    pub title: &'static str,
    /// Brief description.
    pub summary: &'static str,
    /// Link to the original reference.
    pub link: &'static str,
}

/// Link to the BMS command memo.
const MEMO: &str = "https://hitkey.nekokan.dyndns.info/cmdsJP.htm";

/// Header command table.
pub static HEADER_DOCS: &[HeaderDoc] = &[
    HeaderDoc {
        command: "TITLE",
        title: "#TITLE",
        summary: "Song title; shown on the song selection and gameplay screens.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SUBTITLE",
        title: "#SUBTITLE",
        summary: "Song subtitle; may be shown together with #TITLE.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ARTIST",
        title: "#ARTIST",
        summary: "Main artist or composer.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SUBARTIST",
        title: "#SUBARTIST",
        summary: "Co-creator or arranger.",
        link: MEMO,
    },
    HeaderDoc {
        command: "GENRE",
        title: "#GENRE",
        summary: "Music genre; some players also accept #GENLE.",
        link: MEMO,
    },
    HeaderDoc {
        command: "GENLE",
        title: "#GENLE",
        summary: "Common misspelling variant of #GENRE.",
        link: MEMO,
    },
    HeaderDoc {
        command: "MAKER",
        title: "#MAKER",
        summary: "Chart author name.",
        link: MEMO,
    },
    HeaderDoc {
        command: "COMMENT",
        title: "#COMMENT",
        summary: "Description text shown in the song list.",
        link: MEMO,
    },
    HeaderDoc {
        command: "CHARSET",
        title: "#CHARSET",
        summary: "Source file encoding declaration, such as Shift_JIS or UTF-8.",
        link: MEMO,
    },
    HeaderDoc {
        command: "URL",
        title: "%URL",
        summary: "Author website URL.",
        link: MEMO,
    },
    HeaderDoc {
        command: "EMAIL",
        title: "%EMAIL",
        summary: "Author email address.",
        link: MEMO,
    },
    HeaderDoc {
        command: "TEXT",
        title: "#TEXTxx",
        summary: "Timed on-screen text definition; referenced by channel 99.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SONG",
        title: "#SONGxx",
        summary: "Synonym for #TEXTxx.",
        link: MEMO,
    },
    HeaderDoc {
        command: "PLAYER",
        title: "#PLAYER",
        summary: "Player mode: 1 single, 2 couple, 3 double, 4 battle.",
        link: MEMO,
    },
    HeaderDoc {
        command: "RANK",
        title: "#RANK",
        summary: "Judgment width; 0 is strictest and 4 is widest.",
        link: MEMO,
    },
    HeaderDoc {
        command: "DEFEXRANK",
        title: "#DEFEXRANK",
        summary: "Default judgment width relative to NORMAL.",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXRANK",
        title: "#EXRANKxx",
        summary: "Per-key judgment width definition; referenced by channel A0.",
        link: MEMO,
    },
    HeaderDoc {
        command: "TOTAL",
        title: "#TOTAL",
        summary: "Total life gauge increase; affects clear difficulty.",
        link: MEMO,
    },
    HeaderDoc {
        command: "VOLWAV",
        title: "#VOLWAV",
        summary: "Overall volume percentage.",
        link: MEMO,
    },
    HeaderDoc {
        command: "PLAYLEVEL",
        title: "#PLAYLEVEL",
        summary: "Chart difficulty value.",
        link: MEMO,
    },
    HeaderDoc {
        command: "DIFFICULTY",
        title: "#DIFFICULTY",
        summary: "Difficulty category: 1 BEGINNER through 5 INSANE.",
        link: MEMO,
    },
    HeaderDoc {
        command: "OPTION",
        title: "#OPTION",
        summary: "Chart-level option definition.",
        link: MEMO,
    },
    HeaderDoc {
        command: "CHANGEOPTION",
        title: "#CHANGEOPTIONxx",
        summary: "Switches options by position; referenced by channel A6.",
        link: MEMO,
    },
    HeaderDoc {
        command: "LNTYPE",
        title: "#LNTYPE",
        summary: "Long-note type: 1 RDM notation, 2 MGQ notation.",
        link: MEMO,
    },
    HeaderDoc {
        command: "LNOBJ",
        title: "#LNOBJ",
        summary: "Treats the specified WAV ID as a long-note end.",
        link: MEMO,
    },
    HeaderDoc {
        command: "LNMODE",
        title: "#LNMODE",
        summary: "beatoraja long-note mode: LN / CN / HCN.",
        link: MEMO,
    },
    HeaderDoc {
        command: "OCT/FP",
        title: "#OCT/FP",
        summary: "Switches to OCT/FP mode.",
        link: MEMO,
    },
    HeaderDoc {
        command: "OCT",
        title: "#OCT",
        summary: "Channel mapping for OCT/FP mode.",
        link: MEMO,
    },
    HeaderDoc {
        command: "FP",
        title: "#FP",
        summary: "Channel mapping for OCT/FP mode.",
        link: MEMO,
    },
    HeaderDoc {
        command: "PREVIEW",
        title: "#PREVIEW",
        summary: "Preview audio file.",
        link: MEMO,
    },
    HeaderDoc {
        command: "PATH_WAV",
        title: "#PATH_WAV",
        summary: "WAV file search path.",
        link: MEMO,
    },
    HeaderDoc {
        command: "STAGEFILE",
        title: "#STAGEFILE",
        summary: "Gameplay background image.",
        link: MEMO,
    },
    HeaderDoc {
        command: "BANNER",
        title: "#BANNER",
        summary: "Song list banner image.",
        link: MEMO,
    },
    HeaderDoc {
        command: "BACKBMP",
        title: "#BACKBMP",
        summary: "Title screen background image.",
        link: MEMO,
    },
    HeaderDoc {
        command: "CHARFILE",
        title: "#CHARFILE",
        summary: "Character image definition.",
        link: MEMO,
    },
    HeaderDoc {
        command: "END",
        title: "#END",
        summary: "Chart end marker.",
        link: MEMO,
    },
    HeaderDoc {
        command: "BASE",
        title: "#BASE",
        summary: "Index radix: 36 or 62.",
        link: MEMO,
    },
    HeaderDoc {
        command: "BPM",
        title: "#BPM",
        summary: "Initial BPM; with an ID, defines an extended BPM.",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXBPM",
        title: "#EXBPMxx",
        summary: "Extended BPM definition; referenced by channel 08.",
        link: MEMO,
    },
    HeaderDoc {
        command: "BASEBPM",
        title: "#BASEBPM",
        summary: "Reference BPM for automatic HI-SPEED.",
        link: MEMO,
    },
    HeaderDoc {
        command: "STOP",
        title: "#STOPxx",
        summary: "Stop sequence definition; referenced by channel 09.",
        link: MEMO,
    },
    HeaderDoc {
        command: "STP",
        title: "#STP",
        summary: "Absolute-time stop: measure.position milliseconds.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SCROLL",
        title: "#SCROLLxx",
        summary: "Scroll speed multiplier definition; referenced by channel SC.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SPEED",
        title: "#SPEEDxx",
        summary: "Visual note spacing definition; referenced by channel SP.",
        link: MEMO,
    },
    HeaderDoc {
        command: "WAV",
        title: "#WAVxx",
        summary: "Audio resource definition; referenced by BGM, note, and LN channels.",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXWAV",
        title: "#EXWAVxx",
        summary: "Audio definition with pan, volume, and frequency parameters.",
        link: MEMO,
    },
    HeaderDoc {
        command: "WAVCMD",
        title: "#WAVCMD",
        summary: "Applies pitch, volume, or duration commands to a WAV.",
        link: MEMO,
    },
    HeaderDoc {
        command: "CDDA",
        title: "#CDDA",
        summary: "CD-DA track definition.",
        link: MEMO,
    },
    HeaderDoc {
        command: "MIDIFILE",
        title: "#MIDIFILE",
        summary: "MIDI file definition.",
        link: MEMO,
    },
    HeaderDoc {
        command: "BMP",
        title: "#BMPxx",
        summary: "Image resource definition; referenced by BGA channels.",
        link: MEMO,
    },
    HeaderDoc {
        command: "EXBMP",
        title: "#EXBMPxx",
        summary: "Image definition with a transparent color.",
        link: MEMO,
    },
    HeaderDoc {
        command: "BGA",
        title: "#BGAxx",
        summary: "BGA object definition; referenced by channels 04/06/07/0A.",
        link: MEMO,
    },
    HeaderDoc {
        command: "@BGA",
        title: "#@BGAxx",
        summary: "Defines a BGA crop in width/height form.",
        link: MEMO,
    },
    HeaderDoc {
        command: "POORBGA",
        title: "#POORBGA",
        summary: "BGA mode shown on MISS.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SWBGA",
        title: "#SWBGAxx",
        summary: "Frame-by-frame animated BGA definition.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ARGB",
        title: "#ARGBxx",
        summary: "BGA definition with an ARGB color.",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEOFILE",
        title: "#VIDEOFILE",
        summary: "Video background file.",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEOf/s",
        title: "#VIDEOf/s",
        summary: "Video playback frame rate.",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEOCOLORS",
        title: "#VIDEOCOLORS",
        summary: "Video color count.",
        link: MEMO,
    },
    HeaderDoc {
        command: "VIDEODLY",
        title: "#VIDEODLY",
        summary: "Video delay.",
        link: MEMO,
    },
    HeaderDoc {
        command: "MOVIE",
        title: "#MOVIE",
        summary: "Legacy video definition.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SEEK",
        title: "#SEEKxx",
        summary: "SEEK definition; referenced by channel 05.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ExtChr",
        title: "#ExtChr",
        summary: "Extended character definition.",
        link: MEMO,
    },
    HeaderDoc {
        command: "RANDOM",
        title: "#RANDOM",
        summary: "Random block: picks a value in [1, N] for #IF selection.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SETRANDOM",
        title: "#SETRANDOM",
        summary: "Fixed random value; does not call the RNG.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDRANDOM",
        title: "#ENDRANDOM",
        summary: "Ends a #RANDOM block.",
        link: MEMO,
    },
    HeaderDoc {
        command: "IF",
        title: "#IF",
        summary: "Conditional branch inside a random block.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ELSEIF",
        title: "#ELSEIF",
        summary: "Conditional branch when previous branches did not match.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ELSE",
        title: "#ELSE",
        summary: "Branch when no condition matched.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDIF",
        title: "#ENDIF",
        summary: "Ends an #IF chain.",
        link: MEMO,
    },
    HeaderDoc {
        command: "IFEND",
        title: "#IFEND",
        summary: "Alias for #ENDIF.",
        link: MEMO,
    },
    HeaderDoc {
        command: "END IF",
        title: "#END IF",
        summary: "Alias for #ENDIF.",
        link: MEMO,
    },
    HeaderDoc {
        command: "RONDAM",
        title: "#RONDAM",
        summary: "Common misspelling variant of #RANDOM.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SWITCH",
        title: "#SWITCH",
        summary: "Multi-case branch block: picks a value in [1, N].",
        link: MEMO,
    },
    HeaderDoc {
        command: "SETSWITCH",
        title: "#SETSWITCH",
        summary: "Fixed switch value; does not call the RNG.",
        link: MEMO,
    },
    HeaderDoc {
        command: "CASE",
        title: "#CASE",
        summary: "Conditional branch for switch.",
        link: MEMO,
    },
    HeaderDoc {
        command: "SKIP",
        title: "#SKIP",
        summary: "Ends fall-through and jumps to #ENDSW.",
        link: MEMO,
    },
    HeaderDoc {
        command: "DEF",
        title: "#DEF",
        summary: "Default branch for switch.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDSW",
        title: "#ENDSW",
        summary: "Ends a #SWITCH block.",
        link: MEMO,
    },
    HeaderDoc {
        command: "ENDSWITCH",
        title: "#ENDSWITCH",
        summary: "Alias for #ENDSW.",
        link: MEMO,
    },
];

/// Channel table.
pub static CHANNEL_DOCS: &[ChannelDoc] = &[
    ChannelDoc {
        code: "01",
        title: "01 BGM",
        summary: "Background music channel; values reference #WAVxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "02",
        title: "02 Measure length",
        summary: "Measure length multiplier; 1.0 means 4/4.",
        link: MEMO,
    },
    ChannelDoc {
        code: "03",
        title: "03 BPM",
        summary: "BPM change; hexadecimal integer.",
        link: MEMO,
    },
    ChannelDoc {
        code: "04",
        title: "04 BGA BASE",
        summary: "BGA base layer; values reference #BGAxx or #BMPxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "05",
        title: "05 SEEK",
        summary: "SEEK object; values reference #SEEKxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "06",
        title: "06 BGA POOR",
        summary: "BGA layer shown on MISS.",
        link: MEMO,
    },
    ChannelDoc {
        code: "07",
        title: "07 BGA LAYER",
        summary: "BGA overlay layer.",
        link: MEMO,
    },
    ChannelDoc {
        code: "08",
        title: "08 Extended BPM",
        summary: "Values reference #BPMxx / #EXBPMxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "09",
        title: "09 STOP",
        summary: "Values reference #STOPxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "0A",
        title: "0A BGA LAYER2",
        summary: "Second BGA layer.",
        link: MEMO,
    },
    ChannelDoc {
        code: "0B",
        title: "0B BGA BASE opacity",
        summary: "BGA base layer opacity.",
        link: MEMO,
    },
    ChannelDoc {
        code: "0C",
        title: "0C BGA LAYER opacity",
        summary: "BGA overlay layer opacity.",
        link: MEMO,
    },
    ChannelDoc {
        code: "0D",
        title: "0D BGA LAYER2 opacity",
        summary: "Second BGA layer opacity.",
        link: MEMO,
    },
    ChannelDoc {
        code: "0E",
        title: "0E BGA POOR opacity",
        summary: "MISS layer opacity.",
        link: MEMO,
    },
    ChannelDoc {
        code: "97",
        title: "97 BGM volume",
        summary: "BGM channel volume.",
        link: MEMO,
    },
    ChannelDoc {
        code: "98",
        title: "98 KEY volume",
        summary: "Key sound volume.",
        link: MEMO,
    },
    ChannelDoc {
        code: "99",
        title: "99 TEXT",
        summary: "Values reference #TEXTxx / #SONGxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "A0",
        title: "A0 JUDGE",
        summary: "Judge override; the value is a Base36 judge level.",
        link: MEMO,
    },
    ChannelDoc {
        code: "A1",
        title: "A1 BGA ARGB BASE",
        summary: "BGA base layer ARGB.",
        link: MEMO,
    },
    ChannelDoc {
        code: "A2",
        title: "A2 BGA ARGB LAYER",
        summary: "BGA overlay layer ARGB.",
        link: MEMO,
    },
    ChannelDoc {
        code: "A3",
        title: "A3 BGA ARGB LAYER2",
        summary: "Second BGA layer ARGB.",
        link: MEMO,
    },
    ChannelDoc {
        code: "A4",
        title: "A4 BGA ARGB POOR",
        summary: "MISS layer ARGB.",
        link: MEMO,
    },
    ChannelDoc {
        code: "A5",
        title: "A5 BGA KEYBOUND",
        summary: "BGA key binding.",
        link: MEMO,
    },
    ChannelDoc {
        code: "A6",
        title: "A6 OPTION",
        summary: "Values reference #CHANGEOPTIONxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "SC",
        title: "SC SCROLL",
        summary: "Scroll speed multiplier; values reference #SCROLLxx.",
        link: MEMO,
    },
    ChannelDoc {
        code: "SP",
        title: "SP SPEED",
        summary: "Visual note spacing; values reference #SPEEDxx.",
        link: MEMO,
    },
];

/// Look up the description for a header command.
#[must_use]
pub fn header_doc(command: &str) -> Option<&'static HeaderDoc> {
    let raw_command = command.trim_start_matches(['#', '%']);
    HEADER_DOCS
        .iter()
        .filter(|doc| {
            raw_command
                .get(..doc.command.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(doc.command))
                && raw_command.get(doc.command.len()..).is_some_and(|suffix| {
                    suffix.is_empty()
                        || (suffix.len() <= 2
                            && suffix.chars().all(|ch| ch.is_ascii_alphanumeric()))
                })
        })
        .max_by_key(|doc| doc.command.len())
}

/// Return all header command descriptions.
#[must_use]
pub const fn header_completions() -> &'static [HeaderDoc] {
    HEADER_DOCS
}

/// Look up the description for a channel.
#[must_use]
pub fn channel_doc(code: &str) -> Option<ChannelDoc> {
    let normalized = code.trim().to_ascii_uppercase();
    if let Some(doc) = CHANNEL_DOCS
        .iter()
        .find(|doc| doc.code.eq_ignore_ascii_case(&normalized))
    {
        return Some(*doc);
    }
    note_channel_doc(&normalized)
}

/// Return all channel descriptions.
#[must_use]
pub const fn channel_completions() -> &'static [ChannelDoc] {
    CHANNEL_DOCS
}

/// Description for a note channel group.
const fn note_channel_doc(code: &str) -> Option<ChannelDoc> {
    let bytes = code.as_bytes();
    let &[first, second] = bytes else {
        return None;
    };
    if !second.is_ascii_alphanumeric() {
        return None;
    }
    let (title, summary) = match first {
        b'1' => (
            "1P visible notes",
            "1P visible notes; values reference #WAVxx.",
        ),
        b'2' => (
            "2P visible notes",
            "2P visible notes; values reference #WAVxx.",
        ),
        b'3' => (
            "1P invisible notes",
            "1P invisible notes; values reference #WAVxx.",
        ),
        b'4' => (
            "2P invisible notes",
            "2P invisible notes; values reference #WAVxx.",
        ),
        b'5' => ("1P long notes", "1P long notes; values reference #WAVxx."),
        b'6' => ("2P long notes", "2P long notes; values reference #WAVxx."),
        b'D' => ("1P mines", "1P mines; values reference #WAVxx."),
        b'E' => ("2P mines", "2P mines; values reference #WAVxx."),
        _ => return None,
    };
    Some(ChannelDoc {
        code: "1Z",
        title,
        summary,
        link: MEMO,
    })
}
