//! Where a BMS chart comes from.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use bmsrs_bms_source::{Document, DocumentOptions, SourceEncoding};
use bmsrs_pipeline::SourceFormat;

/// Where a BMS chart comes from.
///
/// The input decides the source format: [`Input::file`] derives it from the
/// path extension, while [`Input::text`] and [`Input::bytes`] take it
/// explicitly, because nothing in the content says whether it is `.bms`,
/// `.bme`, or `.bmd`. Every `bms::parse*` entry accepts `impl Into<Input>>`,
/// and a `&Path` converts to [`Input::file`].
///
/// ```
/// use bmsrs::bms::{Input, parse};
/// use bmsrs::pipeline::SourceFormat;
///
/// let parsed = parse(Input::text("#TITLE A\n", SourceFormat::Bms)).expect("valid BMS");
/// assert_eq!(parsed.value.metadata.title.as_deref(), Some("A"));
/// ```
#[derive(Debug, Clone)]
pub struct Input<'a> {
    /// Where the chart comes from.
    source: Source<'a>,
    /// Explicit encoding; `None` auto-detects. Ignored for text.
    encoding: Option<SourceEncoding>,
}

/// The three mutually exclusive sources behind [`Input`].
#[derive(Debug, Clone)]
enum Source<'a> {
    /// Decoded text with a declared format.
    Text {
        /// Chart text.
        text: &'a str,
        /// Declared source format.
        format: SourceFormat,
    },
    /// Raw bytes with a declared format.
    Bytes {
        /// Chart bytes.
        bytes: Cow<'a, [u8]>,
        /// Declared source format.
        format: SourceFormat,
    },
    /// A file path; the format follows the extension.
    File {
        /// Path read when the parse starts.
        path: &'a Path,
    },
}

impl<'a> Input<'a> {
    /// Text with an explicit source format.
    #[must_use]
    pub const fn text(text: &'a str, format: SourceFormat) -> Self {
        Self {
            source: Source::Text { text, format },
            encoding: None,
        }
    }

    /// Bytes with an explicit source format.
    ///
    /// A `Vec<u8>` is moved in without copying; a `&[u8]` is copied once
    /// when the document is built.
    #[must_use]
    pub fn bytes(bytes: impl Into<Cow<'a, [u8]>>, format: SourceFormat) -> Self {
        Self {
            source: Source::Bytes {
                bytes: bytes.into(),
                format,
            },
            encoding: None,
        }
    }

    /// A file path; the format comes from the extension and the bytes are
    /// read when the parse starts.
    #[must_use]
    pub const fn file(path: &'a Path) -> Self {
        Self {
            source: Source::File { path },
            encoding: None,
        }
    }

    /// Override the encoding instead of detecting it; ignored for text.
    #[must_use]
    pub const fn with_encoding(mut self, encoding: SourceEncoding) -> Self {
        self.encoding = Some(encoding);
        self
    }

    /// The format this input declares, or the one derived from the file
    /// extension.
    #[must_use]
    pub fn format(&self) -> SourceFormat {
        match &self.source {
            Source::Text { format, .. } | Source::Bytes { format, .. } => *format,
            Source::File { path } => SourceFormat::from_path(path),
        }
    }

    /// Load the input into a lossless document.
    pub(crate) fn into_document(self) -> std::io::Result<Document> {
        let options = DocumentOptions {
            encoding: self.encoding,
        };
        Ok(match self.source {
            Source::Text { text, .. } => Document::from_text(text),
            Source::Bytes { bytes, .. } => Document::from_bytes_with(bytes.into_owned(), &options),
            Source::File { path } => Document::from_bytes_with(std::fs::read(path)?, &options),
        })
    }
}

impl<'a> From<&'a Path> for Input<'a> {
    fn from(path: &'a Path) -> Self {
        Self::file(path)
    }
}

impl<'a> From<&'a PathBuf> for Input<'a> {
    fn from(path: &'a PathBuf) -> Self {
        Self::file(path)
    }
}
