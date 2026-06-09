//! BMSON deserialization powered by chumsky.
//!
//! Uses a chumsky-based JSON parser for diagnostics and error recovery,
//! then deserializes into [`bmson_def`] types via `serde_json::from_str`
//! (zero-copy when the input JSON is well-formed).
//!
//! # Quick start
//!
//! ```rust
//! # use bmson_de_chumsky::from_str;
//! let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]}}"#;
//! if let Ok(bmson) = from_str(json) {
//!     assert_eq!(bmson.song_info.title, "T");
//! } else {
//!     unreachable!("valid bmson JSON should parse");
//! }
//! ```
//!
//! # Error recovery
//!
//! The raw chumsky JSON parser (with error recovery for trailing commas,
//! missing commas, unmatched brackets) is available via [`json::parse_json`].
//! If `from_str` fails due to minor JSON issues, you can recover the value
//! manually:
//!
//! ```rust
//! # use bmson_de_chumsky::json;
//! let malformed = r#"{"key": "value",}"#; // trailing comma
//! let (value, _errors) = json::parse_json(malformed);
//! if let Some(val) = value {
//!     # let cleaned = serde_json::to_string(&val).unwrap();
//!     let _result = bmson_de_chumsky::from_str(&cleaned);
//! }
//! ```

pub mod error;
pub mod json;

pub use error::BmsonDeError;

use bmson_def::DetectedVersion;

/// Parse a BMSON JSON string into a [`bmson_def::Bmson`].
///
/// This is the primary entry point.  It first runs the chumsky parser for
/// validation, then uses `serde_json::from_str` for the actual zero-copy
/// deserialization into [`bmson_def::Bmson`].
///
/// The returned value borrows from `json` (zero-copy).  If the input is
/// not valid JSON per the ECMA-404 specification (e.g. trailing commas),
/// [`BmsonDeError::JsonParse`] or [`BmsonDeError::Deserialize`] is returned.
///
/// For fine-grained control over JSON parsing (error recovery, custom
/// diagnostics), use [`json::parse_json`] directly, then pass the cleaned
/// result back to this function.
///
/// # Errors
///
/// Returns [`BmsonDeError::JsonParse`] when the chumsky parser cannot
/// produce any output (fatal parse error).
///
/// Returns [`BmsonDeError::UnknownVersion`] when the `"version"` field is
/// present but unrecognised.
///
/// Returns [`BmsonDeError::Deserialize`] when `serde_json` rejects the
/// JSON or a required field is missing.
///
/// Returns [`BmsonDeError::V0Conversion`] when conversion from the legacy
/// v0 format to the unified v2 format fails (e.g. invalid `init_bpm`).
pub fn from_str(json: &str) -> Result<bmson_def::Bmson<'_>, BmsonDeError> {
    // 1. Run chumsky parser for validation.
    let (value, errors) = json::parse_json(json);
    let had_output = value.is_some();

    // 2. If the parser produced no output at all, return a parse error
    //    regardless of how the individual errors were classified.
    if !had_output {
        let (_warnings, _recovered, fatal) = json::classify_errors(errors, false);
        let msg = fatal
            .iter()
            .map(|e| format!("{e}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(BmsonDeError::JsonParse(msg));
    }

    // 3. Detect bmson version from the raw text.
    let version = bmson_def::detect_version(json)?;

    // 4. Deserialize; attach chumsky diagnostics on failure.
    deserialize_by_version(json, version).map_err(|e| {
        if errors.is_empty() {
            return e;
        }
        match e {
            BmsonDeError::Deserialize { version, message } => {
                let diag = errors
                    .iter()
                    .map(|e| format!("{e}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                BmsonDeError::Deserialize {
                    version,
                    message: format!("{message}\n  (parser diagnostics: {diag})"),
                }
            }
            other => other,
        }
    })
}

/// Internal: dispatch to version-specific deserialization.
fn deserialize_by_version<'a>(
    json: &'a str,
    version: DetectedVersion,
) -> Result<bmson_def::Bmson<'a>, BmsonDeError> {
    match version {
        DetectedVersion::V2 => serde_json::from_str::<bmson_def::Bmson<'a>>(json).map_err(|e| {
            BmsonDeError::Deserialize {
                version: "v2.0.0",
                message: e.to_string(),
            }
        }),
        DetectedVersion::V1 => {
            let v1: bmson_def::v1::Bmson<'a> =
                serde_json::from_str(json).map_err(|e| BmsonDeError::Deserialize {
                    version: "v1.0.0",
                    message: e.to_string(),
                })?;
            Ok(bmson_def::Bmson::from(v1))
        }
        DetectedVersion::V0 => {
            let v0: bmson_def::v0::Bmson<'a> =
                serde_json::from_str(json).map_err(|e| BmsonDeError::Deserialize {
                    version: "v0.2.1",
                    message: e.to_string(),
                })?;
            Ok(bmson_def::Bmson::try_from(v0)?)
        }
    }
}
