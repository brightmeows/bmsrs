//! `#bmp`, `#bga` and related visual resource definitions.

use crate::BmsTokenAttr;
use crate::id::{BmpTag, BmsChannelId, SeekTag};

/// Visual resource definition headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefVisual<'a> {
    /// `#BMP{id}`
    #[bms_token("#BMP{id} {filename}")]
    Bmp {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#EXBMP{id}`
    #[bms_token("#EXBMP{id} {filename}")]
    ExBmp {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#BGA{id}`
    #[bms_token("#BGA{id} {filename}")]
    Bga {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#@BGA{id}`
    #[bms_token("#@BGA{id} {filename}")]
    AtBga {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#POORBGA`
    #[bms_token("#POORBGA {value}")]
    PoorBga(&'a str),
    /// `#SWBGA{id}`
    #[bms_token("#SWBGA{id} {filename}")]
    SwBga {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#ARGB{id}`
    #[bms_token("#ARGB{id} {filename}")]
    Argb {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#VIDEOFILE`
    #[bms_token("#VIDEOFILE {value}")]
    VideoFile(&'a str),
    /// `#MOVIE`
    #[bms_token("#MOVIE {value}")]
    Movie(&'a str),
    /// `#SEEK{id}`
    #[bms_token("#SEEK{id} {value}")]
    Seek {
        /// The 2-character index.
        id: BmsChannelId<SeekTag>,
        /// The raw value.
        value: f64,
    },
    /// `#ExtChr`
    #[bms_token("#ExtChr {value}")]
    ExtChr(&'a str),
}
