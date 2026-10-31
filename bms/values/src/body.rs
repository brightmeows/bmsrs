//! Splitting BMS channel bodies into 2-character cells.
//!
//! A BMS channel body is a string of 2-character values. Parsing uses a
//! lenient policy: two consecutive valid Base62 characters form one cell,
//! invalid characters are skipped, and a trailing single character is dropped.
//! The parser, writer, and analysis layers share this module so their chunking
//! implementations cannot produce different position models on malformed
//! bodies.

use std::ops::Range;

use crate::index::is_base62;

/// A 2-character cell in a body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell<'a> {
    /// Zero-based position of the cell in the full body grid (including `"00"`
    /// cells skipped by [`BodyCells::non_zero`]).
    pub index: usize,
    /// Byte range of the cell in the body.
    pub range: Range<usize>,
    /// The cell's 2-character text.
    pub text: &'a str,
}

/// Iterator that splits a BMS channel body using the lenient rule.
///
/// Constructed by [`BodyCells::including_zero`] or [`BodyCells::non_zero`].
/// Construction scans once to determine the total cell count; iteration then
/// allocates nothing.
#[derive(Clone, Debug)]
pub struct BodyCells<'a> {
    /// Body to split.
    body: &'a str,
    /// Whether to keep `"00"` cells.
    include_zero: bool,
    /// Number of cells in the full body.
    total: usize,
    /// Next byte position to scan.
    pos: usize,
    /// Full-grid index of the next cell.
    index: usize,
}

impl<'a> BodyCells<'a> {
    /// Split all cells, including `"00"`.
    #[must_use]
    pub fn including_zero(body: &'a str) -> Self {
        Self::new(body, true)
    }

    /// Split non-`"00"` cells; indices still use the full grid.
    #[must_use]
    pub fn non_zero(body: &'a str) -> Self {
        Self::new(body, false)
    }

    /// Construct with an explicit `"00"` policy.
    fn new(body: &'a str, include_zero: bool) -> Self {
        Self {
            body,
            include_zero,
            total: count_cells(body),
            pos: 0,
            index: 0,
        }
    }

    /// Return the number of cells in the full body (including `"00"`,
    /// regardless of `non_zero`).
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }

    /// Return the underlying body.
    #[must_use]
    pub const fn body(&self) -> &'a str {
        self.body
    }
}

impl<'a> Iterator for BodyCells<'a> {
    type Item = Cell<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.body.as_bytes();
        while let (Some(&first), Some(&second)) = (bytes.get(self.pos), bytes.get(self.pos + 1)) {
            if is_base62(first) && is_base62(second) {
                let range = self.pos..self.pos + 2;
                self.pos += 2;
                let index = self.index;
                self.index += 1;
                let text = self.body.get(range.clone()).unwrap_or_default();
                if !self.include_zero && text == "00" {
                    continue;
                }
                return Some(Cell { index, range, text });
            }
            self.pos += 1;
        }
        None
    }
}

/// Count the complete 2-character cells in the body.
fn count_cells(body: &str) -> usize {
    let bytes = body.as_bytes();
    let mut count = 0;
    let mut pos = 0;
    while let (Some(&first), Some(&second)) = (bytes.get(pos), bytes.get(pos + 1)) {
        if is_base62(first) && is_base62(second) {
            count += 1;
            pos += 2;
        } else {
            pos += 1;
        }
    }
    count
}
