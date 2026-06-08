//! `#bmp`, `#bga` and related visual resource definitions.

use crate::id::{BmpTag, BmsChannelId, SeekTag};

/// Visual resource definition headers.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsHeaderResDefVisual<'a> {
    /// `#BMPxx`
    Bmp {
        /// The 2-character index.
        index: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#EXBMPxx`
    ExBmp {
        /// The 2-character index.
        index: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#BGAxx`
    Bga {
        /// The 2-character index.
        index: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#@BGAxx`
    AtBga {
        /// The 2-character index.
        index: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#POORBGA`
    PoorBga(&'a str),
    /// `#SWBGAxx`
    SwBga {
        /// The 2-character index.
        index: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#ARGBxx`
    Argb {
        /// The 2-character index.
        index: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#VIDEOFILE`
    VideoFile(&'a str),
    /// `#MOVIE`
    Movie(&'a str),
    /// `#SEEKxx`
    Seek {
        /// The 2-character index.
        index: BmsChannelId<SeekTag>,
        /// The raw value string.
        value: f64,
    },
    /// `#ExtChr`
    ExtChr(&'a str),
}
