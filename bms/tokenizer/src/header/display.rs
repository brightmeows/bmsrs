//! `#title`, `#artist`, `#genre` and related display metadata.

/// Display and difficulty headers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeaderDisplay<'a> {
    /// `#STAGEFILE`
    #[serde(borrow)]
    StageFile(&'a str),
    /// `#BANNER`
    #[serde(borrow)]
    Banner(&'a str),
    /// `#BACKBMP`
    #[serde(borrow)]
    BackBmp(&'a str),
    /// `#CHARFILE`
    #[serde(borrow)]
    CharFile(&'a str),
    /// `#PLAYLEVEL`
    #[serde(borrow)]
    PlayLevel(&'a str),
    /// `#DIFFICULTY`
    #[serde(borrow)]
    Difficulty(&'a str),
    /// `#PREVIEW` (beatoraja extension)
    #[serde(borrow)]
    Preview(&'a str),
}
