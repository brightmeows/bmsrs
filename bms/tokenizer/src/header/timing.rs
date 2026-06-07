/// Timing definition headers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeaderTiming<'a> {
    /// `#BPM` (global BPM)
    #[serde(borrow)]
    Bpm(&'a str),
    /// `#BPMxx` with its 2-character index.
    BpmDef {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        #[serde(borrow)]
        index: &'a str,
        /// The raw value string.
        #[serde(borrow)]
        value: &'a str,
    },
    /// `#BASEBPM`
    #[serde(borrow)]
    BaseBpm(&'a str),
    /// `#STOPxx`
    StopDef {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// The raw value string.
        #[serde(borrow)]
        value: &'a str,
    },
    /// `#SCROLLxx`
    ScrollDef {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// The raw value string.
        #[serde(borrow)]
        value: &'a str,
    },
    /// `#SPEEDxx`
    SpeedDef {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// The raw value string.
        #[serde(borrow)]
        value: &'a str,
    },
}
