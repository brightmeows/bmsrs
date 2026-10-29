//! Lossless documents: construction, editing, and write-back.

use crate::classify::{ScanState, classify_line, classify_str, scan_line};
use crate::source::split_lines;
use crate::{
    DetectedEncoding, DetectionMethod, Line, LineEnding, LineKind, SourceEncoding,
    UnmappableCharacter,
};
use std::fmt;

/// Document construction options.
#[derive(Debug, Clone, Default)]
pub struct DocumentOptions {
    /// Explicitly specified encoding; `None` means auto-detect.
    pub encoding: Option<SourceEncoding>,
}

/// A lossless BMS document.
///
/// The document owns the raw bytes and the encoding. Unedited lines are sliced
/// directly during [`render`](Self::render), without decoding or re-encoding,
/// so comments, blank lines, indentation, unknown commands, and original
/// casing are all preserved.
///
/// # Line editing
///
/// - [`replace_line`](Self::replace_line): replace the whole line; trivia
///   inside the old line is discarded with it.
/// - [`replace_content`](Self::replace_content): replace only the content span;
///   the prefix and suffix keep their raw bytes. Degenerates to a whole-line
///   replacement for lines that were already replaced.
/// - [`insert_line`](Self::insert_line): the new line inherits the terminator
///   of a neighboring line.
/// - [`remove_line`](Self::remove_line): remove the whole line.
///
/// When edited content cannot be represented in the original encoding, returns
/// [`EditError::Unmappable`] and leaves the document unchanged. New line
/// content that fails to parse is still accepted and classified as
/// [`LineKind::Error`].
pub struct Document {
    /// Raw bytes.
    bytes: Vec<u8>,
    /// Encoding in use.
    encoding: SourceEncoding,
    /// BOM length in bytes.
    bom_len: usize,
    /// Encoding detection result.
    detection: DetectedEncoding,
    /// Lines.
    lines: Vec<Line>,
}

impl fmt::Debug for Document {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Document")
            .field("bytes", &self.bytes.len())
            .field("encoding", &self.encoding)
            .field("bom_len", &self.bom_len)
            .field("detection", &self.detection)
            .field("lines", &self.lines)
            .finish()
    }
}

impl Document {
    /// Build a document from bytes, detecting the encoding automatically.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bytes_with(bytes, &DocumentOptions::default())
    }

    /// Build a document from bytes and options.
    #[must_use]
    pub fn from_bytes_with(bytes: Vec<u8>, options: &DocumentOptions) -> Self {
        let detection = options.encoding.map_or_else(
            || SourceEncoding::detect(&bytes),
            |encoding| DetectedEncoding {
                encoding,
                bom_len: encoding.bom_len(&bytes),
                declared: None,
                method: DetectionMethod::Explicit,
            },
        );
        let encoding = detection.encoding;
        let bom_len = detection.bom_len;

        let body = bytes.get(bom_len..).unwrap_or(&[]);
        let utf16 = encoding.utf16_little_endian();
        let mut state = ScanState::default();
        let mut lines = Vec::new();

        for span in split_lines(body, encoding) {
            let line_body = body.get(span.body.clone()).unwrap_or(&[]);
            let scanned = scan_line(line_body, utf16, &mut state);
            let kind = classify_line(line_body, &scanned, encoding);
            let line_start = bom_len + span.range.start;
            lines.push(Line {
                span: line_start..bom_len + span.range.end,
                terminator: span.terminator,
                // `scanned.content` is relative to the line body; converting it
                // to document byte offsets must add the line's own start
                // offset. Adding only the BOM would misplace every line after
                // the first.
                content: scanned
                    .content
                    .map(|range| line_start + range.start..line_start + range.end),
                kind,
                replacement: None,
            });
        }

        Self {
            bytes,
            encoding,
            bom_len,
            detection,
            lines,
        }
    }

    /// Build a document from UTF-8 text.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let options = DocumentOptions {
            encoding: Some(SourceEncoding::UTF8),
        };
        Self::from_bytes_with(text.as_bytes().to_vec(), &options)
    }

    /// Return the encoding used by the document.
    #[must_use]
    pub const fn encoding(&self) -> SourceEncoding {
        self.encoding
    }

    /// Return the encoding detection result.
    #[must_use]
    pub const fn detection(&self) -> &DetectedEncoding {
        &self.detection
    }

    /// Return the BOM bytes.
    #[must_use]
    pub fn bom(&self) -> &[u8] {
        self.bytes.get(..self.bom_len).unwrap_or(&[])
    }

    /// Return the lines.
    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Return the number of lines.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.lines.len()
    }

    /// Return whether the document has no lines.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Return the line at the given index.
    #[must_use]
    pub fn line(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    /// Return the text of the given line, excluding the terminator.
    #[must_use]
    pub fn line_text(&self, index: usize) -> Option<String> {
        let line = self.lines.get(index)?;
        let bytes = self.line_bytes(index);
        let body_len = bytes
            .len()
            .saturating_sub(line.terminator.byte_len(self.encoding));
        let body = bytes.get(..body_len)?;
        Some(self.encoding.decode(body).0.into_owned())
    }

    /// Replace a whole line.
    ///
    /// Indentation and inline comments in the old line are discarded; the line
    /// terminator is preserved.
    ///
    /// # Errors
    ///
    /// Returns [`EditError`] when the line index is out of bounds or `text`
    /// cannot be represented in the original encoding.
    pub fn replace_line(&mut self, index: usize, text: &str) -> Result<(), EditError> {
        let Some(line) = self.lines.get(index) else {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        };
        let terminator = line.terminator;
        let mut bytes = self.encoding.encode(text)?;
        terminator.append_to(&mut bytes, self.encoding);
        let kind = classify_text(text);

        if let Some(target) = self.lines.get_mut(index) {
            target.replacement = Some(bytes);
            target.content = None;
            target.kind = kind;
        }
        Ok(())
    }

    /// Replace only the content span, preserving the raw bytes of the prefix
    /// and suffix trivia.
    ///
    /// Degenerates to [`replace_line`](Self::replace_line) for lines that were
    /// replaced as a whole (or have no content span).
    ///
    /// # Errors
    ///
    /// Returns [`EditError`] when the line index is out of bounds or `text`
    /// cannot be represented in the original encoding.
    pub fn replace_content(&mut self, index: usize, text: &str) -> Result<(), EditError> {
        let Some(line) = self.lines.get(index) else {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        };
        if line.replacement.is_some() {
            return self.replace_line(index, text);
        }
        let Some(content) = line.content.clone() else {
            return self.replace_line(index, text);
        };
        let span = line.span.clone();
        let terminator = line.terminator;
        let body_end = span.end.saturating_sub(terminator.byte_len(self.encoding));
        let encoded = self.encoding.encode(text)?;
        let mut bytes = Vec::with_capacity(span.len().saturating_add(encoded.len()));
        bytes.extend_from_slice(self.bytes.get(span.start..content.start).unwrap_or(&[]));
        bytes.extend_from_slice(&encoded);
        bytes.extend_from_slice(self.bytes.get(content.end..body_end).unwrap_or(&[]));
        terminator.append_to(&mut bytes, self.encoding);
        let kind = classify_text(text);

        if let Some(target) = self.lines.get_mut(index) {
            target.replacement = Some(bytes);
            target.kind = kind;
        }
        Ok(())
    }

    /// Insert a line before `index`.
    ///
    /// The new line inherits the terminator of a neighboring line. When
    /// inserting after a final line that has no terminator, the final line
    /// first gets an `LF` so the two lines do not run together.
    ///
    /// # Errors
    ///
    /// Returns [`EditError`] when the line index is greater than the number of
    /// lines or `text` cannot be represented in the original encoding.
    pub fn insert_line(&mut self, index: usize, text: &str) -> Result<(), EditError> {
        if index > self.lines.len() {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        }

        let inherited = index
            .checked_sub(1)
            .and_then(|previous| self.lines.get(previous))
            .map(Line::terminator)
            .or_else(|| self.lines.get(index).map(Line::terminator));
        let terminator = match inherited {
            Some(LineEnding::None) | None => LineEnding::Lf,
            Some(other) => other,
        };

        if let Some(previous) = index.checked_sub(1)
            && self
                .lines
                .get(previous)
                .is_some_and(|line| line.terminator == LineEnding::None)
        {
            let mut bytes = self.line_bytes(previous).to_vec();
            LineEnding::Lf.append_to(&mut bytes, self.encoding);
            if let Some(target) = self.lines.get_mut(previous) {
                target.replacement = Some(bytes);
                target.terminator = LineEnding::Lf;
            }
        }

        let mut bytes = self.encoding.encode(text)?;
        terminator.append_to(&mut bytes, self.encoding);
        let kind = classify_text(text);
        self.lines.insert(
            index,
            Line {
                span: 0..0,
                terminator,
                content: None,
                kind,
                replacement: Some(bytes),
            },
        );
        Ok(())
    }

    /// Remove the line at `index` and return it.
    ///
    /// # Errors
    ///
    /// Returns [`EditError::OutOfBounds`] when the line index is out of bounds.
    pub fn remove_line(&mut self, index: usize) -> Result<Line, EditError> {
        if index >= self.lines.len() {
            return Err(EditError::OutOfBounds {
                index,
                len: self.lines.len(),
            });
        }
        Ok(self.lines.remove(index))
    }

    /// Emit the full file bytes.
    ///
    /// Unedited lines are sliced verbatim; edited lines emit their replacement
    /// bytes.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.bytes.len());
        output.extend_from_slice(self.bom());
        for index in 0..self.lines.len() {
            output.extend_from_slice(self.line_bytes(index));
        }
        output
    }

    /// Emit bytes for 0-based line indices, including the BOM.
    ///
    /// Unedited lines are sliced verbatim; edited lines emit their replacement
    /// bytes. Out-of-bounds indices are skipped.
    #[must_use]
    pub fn render_line_indices(&self, indices: &[usize]) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(self.bom());
        for &index in indices {
            output.extend_from_slice(self.line_bytes(index));
        }
        output
    }

    /// Return the full text of the current document.
    ///
    /// The BOM is removed and line endings are preserved from the original
    /// file; decode errors are represented with replacement characters.
    #[must_use]
    pub fn to_text(&self) -> String {
        let bytes = self.render();
        let body = bytes.get(self.bom_len..).unwrap_or(&[]);
        self.encoding.decode(body).0.into_owned()
    }

    /// Return the full text of the current document; returns
    /// [`TextDecodeError`] when decoding fails.
    ///
    /// # Errors
    ///
    /// Returns an error when the document contains bytes the current encoding
    /// cannot decode.
    pub fn try_to_text(&self) -> Result<String, TextDecodeError> {
        let bytes = self.render();
        let body = bytes.get(self.bom_len..).unwrap_or(&[]);
        let (text, had_errors) = self.encoding.decode(body);
        if had_errors {
            Err(TextDecodeError {
                encoding: self.encoding.name(),
            })
        } else {
            Ok(text.into_owned())
        }
    }

    /// Return the current bytes of the given line, preferring the replacement
    /// bytes after an edit.
    fn line_bytes(&self, index: usize) -> &[u8] {
        self.lines.get(index).map_or(&[], |line| {
            line.replacement
                .as_deref()
                .unwrap_or_else(|| self.bytes.get(line.span.clone()).unwrap_or_default())
        })
    }
}

/// Classify new line text.
fn classify_text(text: &str) -> LineKind {
    let mut state = ScanState::default();
    let scanned = scan_line(text.as_bytes(), None, &mut state);
    classify_str(text, &scanned)
}

/// Line editing error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// Line index out of bounds.
    #[error("line index {index} out of bounds (len {len})")]
    OutOfBounds {
        /// Requested index.
        index: usize,
        /// Number of lines in the document.
        len: usize,
    },
    /// The text cannot be represented in the original encoding.
    #[error(transparent)]
    Unmappable(#[from] UnmappableCharacter),
}

/// Failed to decode the document text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("document contains bytes undecodable with {encoding}")]
pub struct TextDecodeError {
    /// Document encoding name.
    pub encoding: &'static str,
}
