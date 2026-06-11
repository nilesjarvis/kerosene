use super::format_book_size;

#[test]
fn outcome_books_format_level_sizes_as_whole_contracts() {
    assert_eq!(format_book_size(5.0, true), "5");
    assert_eq!(format_book_size(150.0, true), "150");
    assert_eq!(format_book_size(12_345.0, true), "12345");
}

#[test]
fn non_outcome_books_keep_fractional_size_formatting() {
    assert_eq!(format_book_size(5.0, false), "5.00");
    assert_eq!(format_book_size(150.0, false), "150.0");
    assert_eq!(format_book_size(0.1234, false), "0.1234");
    assert_eq!(format_book_size(12_345.0, false), "12.3K");
}
