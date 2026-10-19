use super::{format_book_size, format_book_total};

#[test]
fn order_book_total_formats_usd_with_cents_even_for_whole_contracts() {
    assert_eq!(format_book_total(5.0, true, false), "5");
    assert_eq!(format_book_total(5.0, false, false), "5.00");
    for whole_contracts in [false, true] {
        assert_eq!(format_book_total(0.55, whole_contracts, true), "$0.55");
        assert_eq!(
            format_book_total(1234.5, whole_contracts, true),
            "$1,234.50"
        );
        assert_eq!(format_book_total(12345.0, whole_contracts, true), "$12.3K");
        assert_eq!(format_book_total(1234500.0, whole_contracts, true), "$1.2M");
    }
}

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
