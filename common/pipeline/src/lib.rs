//! Shared pipeline result and diagnostic types.
//!
//! This crate is the dependency-free vocabulary shared by the BMS and BMSON
//! processors and the public facade:
//!
//! - [`Outcome`] pairs a produced value with the diagnostics collected while
//!   producing it.
//! - [`SourceSpan`] is a format-neutral byte range.
//! - [`Diagnostic`], [`Severity`], and [`DiagnosticCode`] describe one
//!   user-facing diagnostic.
//! - [`ErrorStrategy`] selects whether a whole-file parse collects every
//!   error or stops at the first one.
//! - [`SourceFormat`] names the input dialect selected by the entry layer.
//!
//! It intentionally has no dependencies so that `bmsrs-chart` and
//! `bmsrs-player` remain dependency-free as well.

use std::fmt;

/// A produced value plus the diagnostics collected while producing it.
///
/// The severity of each diagnostic travels with the diagnostic itself. An
/// outcome can hold a value while also holding diagnostics of error severity
/// that the producer recovered from, so [`diagnostics`](Self::diagnostics) is
/// not limited to warning severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<T, D> {
    /// Produced value.
    pub value: T,
    /// Diagnostics collected while producing the value.
    pub diagnostics: Vec<D>,
}

impl<T, D> Outcome<T, D> {
    /// Construct an outcome from a value and its diagnostics.
    #[must_use]
    pub const fn new(value: T, diagnostics: Vec<D>) -> Self {
        Self { value, diagnostics }
    }

    /// Split into the value and the diagnostic list.
    #[must_use]
    pub fn into_parts(self) -> (T, Vec<D>) {
        (self.value, self.diagnostics)
    }

    /// Map the value, keeping diagnostics unchanged.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Outcome<U, D> {
        Outcome {
            value: f(self.value),
            diagnostics: self.diagnostics,
        }
    }

    /// Map every diagnostic, keeping the value unchanged.
    #[must_use]
    pub fn map_diagnostics<V>(self, f: impl FnMut(D) -> V) -> Outcome<T, V> {
        Outcome {
            value: self.value,
            diagnostics: self.diagnostics.into_iter().map(f).collect(),
        }
    }
}

impl<T, D: HasSeverity> Outcome<T, D> {
    /// Return whether any diagnostic has error severity.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
    }

    /// Iterate over diagnostics with error severity.
    pub fn errors(&self) -> impl Iterator<Item = &D> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
    }

    /// Convert into a strict result.
    ///
    /// Returns `Err` with every diagnostic, in collection order, when at least
    /// one diagnostic has error severity. Without errors, returns the value
    /// and discards the diagnostics; use [`into_parts`](Self::into_parts) when
    /// the diagnostics are needed on success.
    ///
    /// # Errors
    ///
    /// Returns every collected diagnostic when at least one has error
    /// severity.
    pub fn into_result(self) -> Result<T, Vec<D>> {
        if self.has_errors() {
            Err(self.diagnostics)
        } else {
            Ok(self.value)
        }
    }
}

/// Error-handling strategy for whole-file parsing pipelines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// Collect errors from every line.
    #[default]
    CollectAll,
    /// Return at the first error.
    FailFast,
}

/// A byte range in source text.
///
/// Offsets are half-open: `start` is inclusive and `end` is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SourceSpan {
    /// Start byte offset (inclusive).
    pub start: u32,
    /// End byte offset (exclusive).
    pub end: u32,
}

impl SourceSpan {
    /// Construct a span from half-open byte offsets.
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Construct an empty span at the given offset.
    #[must_use]
    pub const fn empty(offset: u32) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Return the span length in bytes.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Return whether the span is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }

    /// Return whether the span contains the given offset.
    #[must_use]
    pub const fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Error.
    Error,
    /// Warning.
    Warning,
    /// Information.
    Information,
}

/// Diagnostic severity access.
///
/// Implemented by every diagnostic and warning type that can travel in an
/// [`Outcome`]. Keeping the accessor on a trait lets [`Outcome::has_errors`]
/// and [`Outcome::into_result`] work across the pipeline's typed diagnostic
/// shapes.
pub trait HasSeverity {
    /// Return the diagnostic severity.
    fn severity(&self) -> Severity;
}

/// Stable diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCode {
    /// CST syntax error.
    Syntax,
    /// Control-flow structure error.
    ControlFlow,
    /// Single-line tokenization error.
    Tokenize,
    /// Source decoding error.
    Decode,
    /// Duplicate resource definition.
    DuplicateDefinition,
    /// Reference without a definition.
    UndefinedReference,
    /// BMS processing warning.
    BmsProcess,
    /// BMSON processing warning.
    BmsonProcess,
    /// `KaleidBeat` conditional-block structure warning.
    Conditional,
    /// `KaleidBeat` dialect extension kept outside the core chart.
    Extension,
}

impl DiagnosticCode {
    /// Return the stable string form used by protocol adapters.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::ControlFlow => "control-flow",
            Self::Tokenize => "tokenize",
            Self::Decode => "decode",
            Self::DuplicateDefinition => "duplicate-definition",
            Self::UndefinedReference => "undefined-reference",
            Self::BmsProcess => "bms-process",
            Self::BmsonProcess => "bmson-process",
            Self::Conditional => "conditional",
            Self::Extension => "extension",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One user-facing diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Source span, when the diagnostic can be located in the source text.
    pub span: Option<SourceSpan>,
    /// Message.
    pub message: String,
    /// Stable diagnostic code, when one applies.
    pub code: Option<DiagnosticCode>,
}

impl Diagnostic {
    /// Construct a diagnostic with the two fields every diagnostic carries.
    ///
    /// Attach the optional location and code through [`with_span`](Self::with_span)
    /// and [`with_code`](Self::with_code); keeping them out of the argument
    /// list stops a caller from having to read past two positional `Option`s.
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            span: None,
            message: message.into(),
            code: None,
        }
    }

    /// Attach the source span this diagnostic points at.
    #[must_use]
    pub const fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Attach the stable diagnostic code.
    #[must_use]
    pub const fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
    }

    /// Return whether this diagnostic is an error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }
}

impl HasSeverity for Diagnostic {
    fn severity(&self) -> Severity {
        self.severity
    }
}

/// Source format selected by the entry layer.
///
/// `Bms` covers the BMS text family (`.bms`, `.bme`, `.bml`, and extensionless
/// files), `Bmc` is the `KaleidBeat`-recommended marker for dialect charts,
/// `Bmd` is the BMS-derived synthesis format, and `Bmson` is JSON.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SourceFormat {
    /// Standard BMS text family.
    #[default]
    Bms,
    /// `.bmc` dialect marker; parsed exactly like BMS.
    Bmc,
    /// `.bmd` synthesis format.
    Bmd,
    /// `.bmson` JSON.
    Bmson,
}

impl SourceFormat {
    /// Infer the format from a file path's extension.
    ///
    /// `.bmson` and `.json` select [`Bmson`](Self::Bmson); every other
    /// extension, including an unknown one, falls back to [`Bms`](Self::Bms).
    #[must_use]
    pub fn from_path(path: &std::path::Path) -> Self {
        match path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("bmd") => Self::Bmd,
            Some("bmc") => Self::Bmc,
            Some("bmson" | "json") => Self::Bmson,
            _ => Self::Bms,
        }
    }
}
