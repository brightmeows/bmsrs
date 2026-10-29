//! `BodyCells` 公开 API 测试。

use bms_values::BodyCells;

#[test]
fn all_includes_zero() {
    let cells: Vec<_> = BodyCells::including_zero("0100FF").collect();
    assert_eq!(
        cells
            .iter()
            .map(|cell| (cell.index, cell.text))
            .collect::<Vec<_>>(),
        vec![(0, "01"), (1, "00"), (2, "FF")]
    );
}

#[test]
fn non_zero_skips_zero_without_renumbering() {
    let cells: Vec<_> = BodyCells::non_zero("0100FF").collect();
    assert_eq!(
        cells
            .iter()
            .map(|cell| (cell.index, cell.text))
            .collect::<Vec<_>>(),
        vec![(0, "01"), (2, "FF")]
    );
}

#[test]
fn lenient_skips_invalid_chars() {
    let cells: Vec<_> = BodyCells::including_zero("0-01").collect();
    assert_eq!(
        cells
            .iter()
            .map(|cell| (cell.index, cell.text))
            .collect::<Vec<_>>(),
        vec![(0, "01")]
    );
}

#[test]
fn odd_trailing_char_is_dropped() {
    assert_eq!(BodyCells::including_zero("010").count(), 1);
}

#[test]
fn ranges_match_text() {
    let body = "01-02ZZ";
    let cells: Vec<_> = BodyCells::including_zero(body).collect();
    assert!(
        cells
            .iter()
            .all(|cell| body.get(cell.range.clone()) == Some(cell.text))
    );
}

#[test]
fn total_counts_all_cells_including_zero() {
    assert_eq!(BodyCells::including_zero("0100FF").total(), 3);
}

#[test]
fn non_zero_total_still_counts_zero() {
    assert_eq!(BodyCells::non_zero("0100FF").total(), 3);
}

#[test]
fn empty_body_has_zero_total() {
    assert_eq!(BodyCells::including_zero("").total(), 0);
}
