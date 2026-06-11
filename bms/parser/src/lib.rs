//! Parser for BMS (Be-Music Script) format.
//!
//! This crate provides the second stage of the BMS parsing pipeline:
//! converting a flat token stream (no control-flow commands) into a
//! structured [`Bms`] object with typed metadata, resource definitions,
//! timing, and channel messages.

use std::collections::BTreeMap;

use bms_tokenizer::{
    BmpTag, BmsBaseMode, BmsChannelId, BmsHeader, BmsHeaderDisplay, BmsHeaderFallback,
    BmsHeaderGameplay, BmsHeaderMetadata, BmsHeaderResDefAudio, BmsHeaderResDefVisual,
    BmsHeaderTiming, BmsMessage, BmsToken, BpmTag, ChangeOptionTag, ChannelTag, DifficultyLevel,
    ExRankTag, Hex, LnMode, LnObjTag, LnType, PlayerMode, PoorBgaMode, Rank, ScrollTag, SeekTag,
    SpeedTag, StopTag, TextTag, WavTag,
};

/// Structured representation of a BMS file built from a flat token stream.
///
/// All header fields use `Option` (last-wins semantics).  Resource definitions
/// and messages are stored in `BTreeMap`s keyed by their typed channel IDs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bms {
    // Metadata
    /// Song title (`#TITLE`).
    pub title: Option<String>,
    /// Song subtitle (`#SUBTITLE`).
    pub subtitle: Option<String>,
    /// Song artist / composer (`#ARTIST`).
    pub artist: Option<String>,
    /// Co-creators (`#SUBARTIST`).
    pub sub_artist: Option<String>,
    /// Music genre (`#GENRE` / `#GENLE`).
    pub genre: Option<String>,
    /// BMS chart author name (`#MAKER`).
    pub maker: Option<String>,
    /// Text shown in song-selection list (`#COMMENT`).
    pub comment: Option<String>,
    /// Character encoding hint (`#CHARSET`).
    pub charset: Option<String>,
    /// Author's website URL (`%URL`).
    pub url: Option<String>,
    /// Author's email address (`%EMAIL`).
    pub email: Option<String>,

    // Gameplay
    /// Game mode (`#PLAYER`).
    pub player: Option<PlayerMode>,
    /// Judgment difficulty (`#RANK`).
    pub rank: Option<Rank>,
    /// Fine-grained judgment difficulty as percentage (`#DEFEXRANK`).
    pub def_ex_rank: Option<f64>,
    /// Maximum groove gauge increase (`#TOTAL`).
    pub total: Option<f64>,
    /// Master volume percentage (`#VOLWAV`).
    pub vol_wav: Option<f64>,
    /// Long-note notation (`#LNTYPE`).
    pub ln_type: Option<LnType>,
    /// WAV index used as LN termination marker (`#LNOBJ`).
    pub ln_obj: Option<BmsChannelId<LnObjTag>>,
    /// Forced LN / CN / HCN mode (`#LNMODE`).
    pub ln_mode: Option<LnMode>,
    /// Numbering base for indexed commands (`#BASE`).
    pub base: Option<BmsBaseMode>,

    // Resource definitions
    /// Sound effect / BGM file definitions (`#WAV`, `#EXWAV`).
    pub wav_files: BTreeMap<BmsChannelId<WavTag>, String>,
    /// Image file definitions (`#BMP`).
    pub bmp_files: BTreeMap<BmsChannelId<BmpTag>, String>,
    /// Extended BPM definitions (`#BPMxx`, `#EXBPMxx`).
    pub bpm_defs: BTreeMap<BmsChannelId<BpmTag>, f64>,
    /// Stop-sequence definitions (`#STOPxx`).
    pub stop_defs: BTreeMap<BmsChannelId<StopTag>, f64>,
    /// Scroll speed multiplier definitions (`#SCROLLxx`).
    pub scroll_defs: BTreeMap<BmsChannelId<ScrollTag>, f64>,
    /// Visual note-spacing definitions (`#SPEEDxx`).
    pub speed_defs: BTreeMap<BmsChannelId<SpeedTag>, f64>,
    /// Per-position judgment width overrides (`#EXRANKxx`).
    pub ex_rank_defs: BTreeMap<BmsChannelId<ExRankTag>, f64>,
    /// Timed on-screen text definitions (`#TEXTxx`, `#SONGxx`).
    pub text_defs: BTreeMap<BmsChannelId<TextTag>, String>,
    /// Dynamic option-change definitions (`#CHANGEOPTIONxx`).
    pub change_option_defs: BTreeMap<BmsChannelId<ChangeOptionTag>, String>,
    /// Video seek position definitions (`#SEEKxx`).
    pub seek_defs: BTreeMap<BmsChannelId<SeekTag>, f64>,

    // Display
    /// Splash-screen image (`#STAGEFILE`).
    pub stage_file: Option<String>,
    /// Banner image (`#BANNER`).
    pub banner: Option<String>,
    /// Background image (`#BACKBMP`).
    pub back_bmp: Option<String>,
    /// Character animation file (`#CHARFILE`).
    pub char_file: Option<String>,
    /// Difficulty number (`#PLAYLEVEL`).
    pub play_level: Option<f64>,
    /// Difficulty category 1–5 (`#DIFFICULTY`).
    pub difficulty: Option<DifficultyLevel>,
    /// Preview audio file (`#PREVIEW`).
    pub preview: Option<String>,
    /// Poor/miss BGA display mode (`#POORBGA`).
    pub poor_bga_mode: Option<PoorBgaMode>,

    // Timing
    /// Global initial BPM (`#BPM`).
    pub bpm: Option<f64>,
    /// Reference BPM for auto HI-SPEED (`#BASEBPM`).
    pub base_bpm: Option<f64>,

    // Messages
    /// Channel data lines keyed by (measure → channel → values).
    pub messages: BTreeMap<u16, BTreeMap<BmsChannelId<ChannelTag, Hex>, String>>,

    // Video/other
    /// Video file as BGA (`#VIDEOFILE`).
    pub video_file: Option<String>,
    /// Video file as BGA, no loop (`#MOVIE`).
    pub movie: Option<String>,
    /// Directory prefix for audio file lookup (`#PATH_WAV`).
    pub path_wav: Option<String>,

    // Fallback
    /// Unrecognised / engine-specific headers preserved as (command, value).
    pub fallback_headers: Vec<(String, String)>,
}

impl Bms {
    /// Build a `Bms` from a flat token stream (no control-flow commands).
    ///
    /// Iterates over tokens and populates fields.  Headers use last-wins
    /// semantics; messages are stored per (measure, channel) pair.
    pub fn from_flat_tokens<'a>(tokens: impl IntoIterator<Item = BmsToken<'a>>) -> Self {
        let mut bms = Self::default();

        for token in tokens {
            match token {
                BmsToken::Header(header) => bms.process_header(&header),
                BmsToken::Message(msg) => bms.process_message(&msg),
            }
        }

        bms
    }

    /// Dispatch a header to the appropriate handler by domain.
    fn process_header(&mut self, header: &BmsHeader<'_>) {
        match header {
            BmsHeader::Metadata(m) => self.process_metadata(m),
            BmsHeader::Gameplay(g) => self.process_gameplay(g),
            BmsHeader::Timing(t) => self.process_timing(t),
            BmsHeader::ResDefAudio(a) => self.process_res_def_audio(a),
            BmsHeader::ResDefVisual(v) => self.process_res_def_visual(v),
            BmsHeader::Display(d) => self.process_display(d),
            BmsHeader::ControlFlow(_) => { /* skipped: no control-flow in flat stream */ }
            BmsHeader::Fallback(f) => self.process_fallback(f),
        }
    }

    /// Populate metadata fields from a metadata header.
    fn process_metadata(&mut self, m: &BmsHeaderMetadata<'_>) {
        match m {
            BmsHeaderMetadata::Title(s) => self.title = Some((*s).to_owned()),
            BmsHeaderMetadata::Subtitle(s) => self.subtitle = Some((*s).to_owned()),
            BmsHeaderMetadata::Artist(s) => self.artist = Some((*s).to_owned()),
            BmsHeaderMetadata::SubArtist(s) => self.sub_artist = Some((*s).to_owned()),
            BmsHeaderMetadata::Genre(s) => self.genre = Some((*s).to_owned()),
            BmsHeaderMetadata::Maker(s) => self.maker = Some((*s).to_owned()),
            BmsHeaderMetadata::Comment(s) => self.comment = Some((*s).to_owned()),
            BmsHeaderMetadata::Charset(s) => self.charset = Some((*s).to_owned()),
            BmsHeaderMetadata::Url(s) => self.url = Some((*s).to_owned()),
            BmsHeaderMetadata::Email(s) => self.email = Some((*s).to_owned()),
            BmsHeaderMetadata::Text { id, value } => {
                self.text_defs.insert(*id, (*value).to_owned());
            }
        }
    }

    /// Populate gameplay fields from a gameplay header.
    fn process_gameplay(&mut self, g: &BmsHeaderGameplay<'_>) {
        match g {
            BmsHeaderGameplay::Player(m) => self.player = Some(*m),
            BmsHeaderGameplay::Rank(r) => self.rank = Some(*r),
            BmsHeaderGameplay::DefExRank(v) => self.def_ex_rank = Some(*v),
            BmsHeaderGameplay::ExRank { id, value } => {
                self.ex_rank_defs.insert(*id, *value);
            }
            BmsHeaderGameplay::Total(v) => self.total = Some(*v),
            BmsHeaderGameplay::VolWav(v) => self.vol_wav = Some(*v),
            BmsHeaderGameplay::LnType(t) => self.ln_type = Some(*t),
            BmsHeaderGameplay::LnObj(id) => self.ln_obj = Some(*id),
            BmsHeaderGameplay::LnMode(m) => self.ln_mode = Some(*m),
            BmsHeaderGameplay::Base(m) => self.base = Some(*m),
            BmsHeaderGameplay::OctFp | BmsHeaderGameplay::Option(_) => { /* no Bms field */ }
            BmsHeaderGameplay::ChangeOption { id, value } => {
                self.change_option_defs.insert(*id, (*value).to_owned());
            }
        }
    }

    /// Populate timing fields from a timing header.
    fn process_timing(&mut self, t: &BmsHeaderTiming) {
        match t {
            BmsHeaderTiming::Bpm(v) => self.bpm = Some(*v),
            BmsHeaderTiming::BpmDef { id, value } | BmsHeaderTiming::ExBpm { id, value } => {
                self.bpm_defs.insert(*id, *value);
            }
            BmsHeaderTiming::BaseBpm(v) => self.base_bpm = Some(*v),
            BmsHeaderTiming::StopDef { id, value } => {
                self.stop_defs.insert(*id, *value);
            }
            BmsHeaderTiming::ScrollDef { id, value } => {
                self.scroll_defs.insert(*id, *value);
            }
            BmsHeaderTiming::SpeedDef { id, value } => {
                self.speed_defs.insert(*id, *value);
            }
            BmsHeaderTiming::Stp { .. } => { /* position-based stops need special handling */ }
        }
    }

    /// Populate audio resource definitions from an audio header.
    fn process_res_def_audio(&mut self, a: &BmsHeaderResDefAudio<'_>) {
        match a {
            BmsHeaderResDefAudio::Wav { id, filename } => {
                self.wav_files.insert(*id, (*filename).to_owned());
            }
            BmsHeaderResDefAudio::ExWav { id, params } => {
                self.wav_files.insert(*id, params.filename.to_owned());
            }
            BmsHeaderResDefAudio::PathWav(s) => self.path_wav = Some((*s).to_owned()),
            BmsHeaderResDefAudio::WavCmd(_)
            | BmsHeaderResDefAudio::Cdda(_)
            | BmsHeaderResDefAudio::Midifile(_) => { /* no Bms field */ }
        }
    }

    /// Populate visual resource definitions from a visual header.
    fn process_res_def_visual(&mut self, v: &BmsHeaderResDefVisual<'_>) {
        match v {
            BmsHeaderResDefVisual::Bmp { id, filename } => {
                self.bmp_files.insert(*id, (*filename).to_owned());
            }
            BmsHeaderResDefVisual::VideoFile(s) => self.video_file = Some((*s).to_owned()),
            BmsHeaderResDefVisual::Movie(s) => self.movie = Some((*s).to_owned()),
            BmsHeaderResDefVisual::Seek { id, value } => {
                self.seek_defs.insert(*id, *value);
            }
            BmsHeaderResDefVisual::PoorBga(m) => self.poor_bga_mode = Some(*m),
            BmsHeaderResDefVisual::ExBmp { .. }
            | BmsHeaderResDefVisual::Bga { .. }
            | BmsHeaderResDefVisual::AtBga { .. }
            | BmsHeaderResDefVisual::SwBga { .. }
            | BmsHeaderResDefVisual::Argb { .. }
            | BmsHeaderResDefVisual::ExtChr(_)
            | BmsHeaderResDefVisual::VideoFps(_)
            | BmsHeaderResDefVisual::VideoColors(_)
            | BmsHeaderResDefVisual::VideoDly(_) => { /* no Bms field yet */ }
        }
    }

    /// Populate display fields from a display header.
    fn process_display(&mut self, d: &BmsHeaderDisplay<'_>) {
        match d {
            BmsHeaderDisplay::StageFile(s) => self.stage_file = Some((*s).to_owned()),
            BmsHeaderDisplay::Banner(s) => self.banner = Some((*s).to_owned()),
            BmsHeaderDisplay::BackBmp(s) => self.back_bmp = Some((*s).to_owned()),
            BmsHeaderDisplay::CharFile(s) => self.char_file = Some((*s).to_owned()),
            BmsHeaderDisplay::PlayLevel(v) => self.play_level = Some(*v),
            BmsHeaderDisplay::Difficulty(d) => self.difficulty = Some(*d),
            BmsHeaderDisplay::Preview(s) => self.preview = Some((*s).to_owned()),
        }
    }

    /// Store an unrecognised header in the fallback list.
    fn process_fallback(&mut self, f: &BmsHeaderFallback<'_>) {
        self.fallback_headers
            .push((f.command.to_owned(), f.value.to_owned()));
    }

    /// Insert a channel message into the messages map.
    fn process_message(&mut self, m: &BmsMessage<'_>) {
        self.messages
            .entry(m.measure)
            .or_default()
            .insert(m.channel, m.values.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_tokenizer::BmsTokenizer;

    fn parse_tokens(input: &str) -> Vec<BmsToken<'_>> {
        BmsTokenizer::new()
            .tokenize::<Vec<_>>(input)
            .into_iter()
            .filter_map(|(_, res)| res.ok())
            .collect()
    }

    #[test]
    fn header_override_last_wins() {
        let tokens = parse_tokens("#TITLE First\n#TITLE Second");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.title.as_deref(), Some("Second"));
    }

    #[test]
    fn wav_definitions_stored() {
        let tokens = parse_tokens("#WAV01 a.wav\n#WAV02 b.wav");
        let bms = Bms::from_flat_tokens(tokens);
        let id1: BmsChannelId<WavTag> = "01".try_into().unwrap();
        let id2: BmsChannelId<WavTag> = "02".try_into().unwrap();
        assert_eq!(bms.wav_files.get(&id1).map(String::as_str), Some("a.wav"));
        assert_eq!(bms.wav_files.get(&id2).map(String::as_str), Some("b.wav"));
    }

    #[test]
    fn message_storage() {
        let tokens = parse_tokens("#00101:1122");
        let bms = Bms::from_flat_tokens(tokens);
        let ch: BmsChannelId<ChannelTag, Hex> = "01".try_into().unwrap();
        let measure_map = bms.messages.get(&1);
        assert!(measure_map.is_some());
        assert_eq!(
            measure_map.and_then(|m| m.get(&ch).map(String::as_str)),
            Some("1122")
        );
    }

    #[test]
    fn message_override_last_wins() {
        let tokens = parse_tokens("#00101:1122\n#00101:3344");
        let bms = Bms::from_flat_tokens(tokens);
        let ch: BmsChannelId<ChannelTag, Hex> = "01".try_into().unwrap();
        let measure_map = bms.messages.get(&1);
        assert_eq!(
            measure_map.and_then(|m| m.get(&ch).map(String::as_str)),
            Some("3344")
        );
    }

    #[test]
    fn fallback_headers_stored() {
        let tokens = parse_tokens("#MYEXT abc123");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.fallback_headers.len(), 1);
        assert_eq!(
            bms.fallback_headers[0],
            ("MYEXT".to_owned(), "abc123".to_owned())
        );
    }

    #[test]
    fn mixed_headers_and_messages() {
        let tokens = parse_tokens(
            "#TITLE My Song\n#ARTIST composer\n#BPM 180\n#WAV01 kick.wav\n#00111:11223344",
        );
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.title.as_deref(), Some("My Song"));
        assert_eq!(bms.artist.as_deref(), Some("composer"));
        assert_eq!(bms.bpm, Some(180.0));
        let wav_id: BmsChannelId<WavTag> = "01".try_into().unwrap();
        assert_eq!(
            bms.wav_files.get(&wav_id).map(String::as_str),
            Some("kick.wav")
        );
        let ch: BmsChannelId<ChannelTag, Hex> = "11".try_into().unwrap();
        assert_eq!(
            bms.messages
                .get(&1)
                .and_then(|m| m.get(&ch).map(String::as_str)),
            Some("11223344")
        );
    }

    #[test]
    fn bpm_and_bpm_def_separate_fields() {
        let tokens = parse_tokens("#BPM 120\n#BPM01 180.0\n#EXBPM02 200.0");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.bpm, Some(120.0));
        let id1: BmsChannelId<BpmTag> = "01".try_into().unwrap();
        let id2: BmsChannelId<BpmTag> = "02".try_into().unwrap();
        assert_eq!(bms.bpm_defs.get(&id1), Some(&180.0));
        assert_eq!(bms.bpm_defs.get(&id2), Some(&200.0));
    }

    #[test]
    fn default_is_all_none_empty() {
        let bms = Bms::default();
        assert!(bms.title.is_none());
        assert!(bms.artist.is_none());
        assert!(bms.bpm.is_none());
        assert!(bms.wav_files.is_empty());
        assert!(bms.messages.is_empty());
        assert!(bms.fallback_headers.is_empty());
    }
}
