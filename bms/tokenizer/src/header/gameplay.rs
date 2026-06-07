//! `#difficulty`, `#total`, `#rank`, `#bpm`, `#exbpm` and related gameplay parameters.

/// Gameplay behaviour headers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeaderGameplay<'a> {
    /// `#PLAYER`
    #[serde(borrow)]
    Player(&'a str),
    /// `#RANK`
    #[serde(borrow)]
    Rank(&'a str),
    /// `#DEFEXRANK`
    #[serde(borrow)]
    DefExRank(&'a str),
    /// `#EXRANKxx` with its 2-character index.
    ExRank {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        #[serde(borrow)]
        index: &'a str,
        /// The raw value string.
        #[serde(borrow)]
        value: &'a str,
    },
    /// `#TOTAL`
    #[serde(borrow)]
    Total(&'a str),
    /// `#VOLWAV`
    #[serde(borrow)]
    VolWav(&'a str),
    /// `#LNTYPE`
    #[serde(borrow)]
    LnType(&'a str),
    /// `#LNOBJ`
    #[serde(borrow)]
    LnObj(&'a str),
    /// `#LNMODE` (beatoraja extension)
    #[serde(borrow)]
    LnMode(&'a str),
    /// `#OCT`
    #[serde(borrow)]
    Oct(&'a str),
    /// `#FP`
    #[serde(borrow)]
    Fp(&'a str),
    /// `#OPTION`
    #[serde(borrow)]
    Option(&'a str),
    /// `#CHANGEOPTION`
    #[serde(borrow)]
    ChangeOption(&'a str),
}
