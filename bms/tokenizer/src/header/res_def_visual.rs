/// Visual resource definition headers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeaderResDefVisual<'a> {
    /// `#BMPxx`
    Bmp {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#EXBMPxx`
    ExBmp {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#BGAxx`
    Bga {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#@BGAxx`
    AtBga {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#POORBGA`
    #[serde(borrow)]
    PoorBga(&'a str),
    /// `#SWBGAxx`
    SwBga {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#ARGBxx`
    Argb {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#VIDEOFILE`
    #[serde(borrow)]
    VideoFile(&'a str),
    /// `#MOVIE`
    #[serde(borrow)]
    Movie(&'a str),
    /// `#SEEKxx`
    Seek {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// The raw value string.
        #[serde(borrow)]
        value: &'a str,
    },
    /// `#ExtChr`
    #[serde(borrow)]
    ExtChr(&'a str),
}
