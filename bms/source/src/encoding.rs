//! Source-text encodings: validation, detection, and per-line encoding and
//! decoding.

use std::borrow::Cow;

use encoding_rs::Encoding;

/// A source-text encoding that can be used for editing and write-back.
///
/// Construction validates two requirements:
///
/// - **ASCII compatibility** (except UTF-16): line terminators `CR` / `LF`
///   never appear in the trailing byte of a multi-byte character, so lines can
///   be split on byte boundaries.
/// - **Statelessness**: a single line can be re-encoded independently, without
///   replaying escape state for the whole file.
///
/// The `replacement` encoding cannot decode, and `ISO-2022-JP` escape sequences
/// span lines; both are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceEncoding(&'static Encoding);

impl SourceEncoding {
    /// Internal UTF-8 constant (construction always succeeds).
    pub(crate) const UTF8: Self = Self(encoding_rs::UTF_8);

    /// Validate and wrap an encoding.
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedEncoding`] when the encoding is `replacement`,
    /// `ISO-2022-JP`, or neither ASCII-compatible nor UTF-16.
    pub fn new(encoding: &'static Encoding) -> Result<Self, UnsupportedEncoding> {
        if encoding == encoding_rs::REPLACEMENT || encoding == encoding_rs::ISO_2022_JP {
            return Err(UnsupportedEncoding {
                name: encoding.name(),
            });
        }
        let supported = encoding == encoding_rs::UTF_16LE
            || encoding == encoding_rs::UTF_16BE
            || encoding.is_ascii_compatible();
        if supported {
            Ok(Self(encoding))
        } else {
            Err(UnsupportedEncoding {
                name: encoding.name(),
            })
        }
    }

    /// Construct an encoding from a WHATWG encoding label (for example, a
    /// `#CHARSET` value).
    #[must_use]
    pub fn from_label(label: &[u8]) -> Option<Self> {
        let encoding = Encoding::for_label_no_replacement(label)?;
        Self::new(encoding).ok()
    }

    /// Detect the encoding of a byte sequence.
    ///
    /// Detection order:
    ///
    /// 1. BOM
    /// 2. `#CHARSET` declaration
    /// 3. Strict UTF-8
    /// 4. Candidate table: `Shift_JIS`, `EUC-KR`, `GBK`, `Big5`
    ///
    /// When the `#CHARSET` declaration does not match the actual bytes,
    /// detection falls back to the candidate table and records
    /// [`DetectionMethod::DeclaredFallback`] in [`DetectedEncoding::method`].
    #[must_use]
    pub fn detect(bytes: &[u8]) -> DetectedEncoding {
        if let Some((bom_encoding, bom_len)) = Encoding::for_bom(bytes)
            && let Ok(encoding) = Self::new(bom_encoding)
        {
            return DetectedEncoding {
                encoding,
                bom_len,
                declared: None,
                method: DetectionMethod::Bom,
            };
        }

        let declared =
            scan_charset_header(bytes).and_then(|label| Self::from_label(label.as_bytes()));

        if let Some(encoding) = declared {
            if !encoding.decode(bytes).1 {
                return DetectedEncoding {
                    encoding,
                    bom_len: 0,
                    declared: Some(encoding),
                    method: DetectionMethod::Charset,
                };
            }
            return Self::detect_after_declared(bytes, encoding);
        }

        Self::detect_without_declared(bytes)
    }

    /// Fallback detection when the declaration does not match.
    fn detect_after_declared(bytes: &[u8], declared: Self) -> DetectedEncoding {
        if std::str::from_utf8(bytes).is_ok() {
            return DetectedEncoding {
                encoding: Self::UTF8,
                bom_len: 0,
                declared: Some(declared),
                method: DetectionMethod::DeclaredFallback,
            };
        }
        if let Some(encoding) = Self::first_candidate(bytes) {
            return DetectedEncoding {
                encoding,
                bom_len: 0,
                declared: Some(declared),
                method: DetectionMethod::DeclaredFallback,
            };
        }
        DetectedEncoding {
            encoding: Self::UTF8,
            bom_len: 0,
            declared: Some(declared),
            method: DetectionMethod::Lossy,
        }
    }

    /// Detection when there is no declaration.
    fn detect_without_declared(bytes: &[u8]) -> DetectedEncoding {
        if std::str::from_utf8(bytes).is_ok() {
            return DetectedEncoding {
                encoding: Self::UTF8,
                bom_len: 0,
                declared: None,
                method: DetectionMethod::Utf8,
            };
        }
        if let Some(encoding) = Self::first_candidate(bytes) {
            return DetectedEncoding {
                encoding,
                bom_len: 0,
                declared: None,
                method: DetectionMethod::Heuristic,
            };
        }
        DetectedEncoding {
            encoding: Self::UTF8,
            bom_len: 0,
            declared: None,
            method: DetectionMethod::Lossy,
        }
    }

    /// Try the candidate encodings in order and return the first one that
    /// decodes without loss.
    fn first_candidate(bytes: &[u8]) -> Option<Self> {
        let candidates = [
            encoding_rs::SHIFT_JIS,
            encoding_rs::EUC_KR,
            encoding_rs::GBK,
            encoding_rs::BIG5,
        ];
        candidates.into_iter().find_map(|candidate| {
            let encoding = Self::new(candidate).ok()?;
            if encoding.decode(bytes).1 {
                None
            } else {
                Some(encoding)
            }
        })
    }

    /// Return the underlying encoding.
    #[must_use]
    pub const fn encoding(self) -> &'static Encoding {
        self.0
    }

    /// Return the WHATWG canonical name (for example, `"Shift_JIS"`).
    #[must_use]
    pub fn name(self) -> &'static str {
        self.0.name()
    }

    /// Return whether this is UTF-16LE or UTF-16BE.
    #[must_use]
    pub fn is_utf16(self) -> bool {
        self.0 == encoding_rs::UTF_16LE || self.0 == encoding_rs::UTF_16BE
    }

    /// Return the UTF-16 endianness: `Some(true)` for little-endian,
    /// `Some(false)` for big-endian, and `None` for other encodings.
    #[must_use]
    pub fn utf16_little_endian(self) -> Option<bool> {
        if self.0 == encoding_rs::UTF_16LE {
            Some(true)
        } else if self.0 == encoding_rs::UTF_16BE {
            Some(false)
        } else {
            None
        }
    }

    /// Decode bytes and return `(text, had_decode_errors)`.
    ///
    /// The caller is responsible for stripping the BOM first.
    #[must_use]
    pub fn decode(self, bytes: &[u8]) -> (Cow<'_, str>, bool) {
        self.0.decode_without_bom_handling(bytes)
    }

    /// Encode text as bytes.
    ///
    /// # Errors
    ///
    /// Returns [`UnmappableCharacter`] when the text contains characters this
    /// encoding cannot represent. Unlike [`Encoding::encode`], it does not
    /// replace unmappable characters with numeric character references.
    pub fn encode(self, text: &str) -> Result<Vec<u8>, UnmappableCharacter> {
        let (bytes, _, had_unmappable) = self.0.encode(text);
        if had_unmappable {
            Err(UnmappableCharacter {
                encoding: self.0.name(),
            })
        } else {
            Ok(bytes.into_owned())
        }
    }

    /// Return the length of this encoding's BOM at the start of the file.
    pub(crate) fn bom_len(self, bytes: &[u8]) -> usize {
        match Encoding::for_bom(bytes) {
            Some((encoding, len)) if encoding == self.0 => len,
            _ => 0,
        }
    }
}

/// Encoding detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectedEncoding {
    /// Encoding finally selected.
    pub encoding: SourceEncoding,
    /// BOM length in bytes; `0` means no BOM.
    pub bom_len: usize,
    /// Encoding declared by `#CHARSET` in the file, if any.
    pub declared: Option<SourceEncoding>,
    /// Detection path.
    pub method: DetectionMethod,
}

/// Encoding detection path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMethod {
    /// Determined by the BOM.
    Bom,
    /// Determined by the `#CHARSET` declaration, and decoding had no errors.
    Charset,
    /// Strict UTF-8.
    Utf8,
    /// Matched a candidate encoding.
    Heuristic,
    /// The `#CHARSET` declaration failed to decode; fell back to the candidate
    /// table.
    DeclaredFallback,
    /// No candidate succeeded; decoded as UTF-8 lossily.
    Lossy,
    /// Explicitly specified by the caller.
    Explicit,
}

/// The encoding is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unsupported encoding: {name}")]
pub struct UnsupportedEncoding {
    /// Name of the rejected encoding.
    pub name: &'static str,
}

/// The text contains characters the target encoding cannot represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("text cannot be represented in {encoding}")]
pub struct UnmappableCharacter {
    /// Target encoding name.
    pub encoding: &'static str,
}

/// Scan for a `#CHARSET` or `%CHARSET` directive at the start of a line.
///
/// Matches only at line start (`input[0]`, or after a `\n` / `\r` byte) to
/// avoid matching content inside comments or channel data.
fn scan_charset_header(input: &[u8]) -> Option<String> {
    let mut i = 0;
    while let Some(&prefix_byte) = input.get(i) {
        let is_line_start = i == 0 || input.get(i - 1).is_some_and(|&b| b == b'\n' || b == b'\r');

        if !is_line_start || (prefix_byte != b'#' && prefix_byte != b'%') {
            i += 1;
            continue;
        }

        let Some(after_prefix) = input.get(i + 1..) else {
            break;
        };
        if !after_prefix
            .get(..7)
            .is_some_and(|s| s.eq_ignore_ascii_case(b"CHARSET"))
        {
            i += 1;
            continue;
        }

        let Some(after_keyword) = after_prefix.get(7..) else {
            i += 1;
            continue;
        };
        // Skip a line whose value is empty or all whitespace, and keep scanning
        // for later #CHARSET lines.
        let Some(value_start) = after_keyword
            .iter()
            .position(|&byte| !byte.is_ascii_whitespace())
        else {
            i += 1;
            continue;
        };
        let Some(remaining) = after_keyword.get(value_start..) else {
            i += 1;
            continue;
        };
        let value_end = remaining
            .iter()
            .position(|&byte| byte == b'\n' || byte == b'\r')
            .unwrap_or(remaining.len());
        if value_end == 0 {
            i += 1;
            continue;
        }
        let Some(value_bytes) = remaining.get(..value_end) else {
            i += 1;
            continue;
        };
        // Skip values that are not UTF-8 instead of aborting the scan.
        if let Ok(value) = std::str::from_utf8(value_bytes)
            && !value.is_empty()
        {
            return Some(value.to_owned());
        }

        i += 1;
    }
    None
}
