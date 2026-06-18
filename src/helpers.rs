mod aggregation;
mod formatting;
mod order_book;
mod symbols;
mod ui;

pub(crate) use aggregation::{add_optional_f64, positive_percent_change, sum_optional_f64};
pub use formatting::{
    ellipsized_text, fallback_initials, finite_value, format_decimal_with_commas, format_duration,
    format_price, format_relative_time, format_seen_latency_label, format_signed_percent_value,
    format_size, format_timestamp, format_timestamp_exact, format_usd, format_with_commas,
    invalid_data_placeholder, normalize_two_decimal_display_value, not_available_placeholder,
    parse_finite_json_number, parse_finite_number, parse_number, parse_positive_finite_number,
    parse_positive_number, positive_finite_value, redact_sensitive_response_text, response_excerpt,
    response_snippet, sensitive_response_excerpt, sensitive_response_snippet, text_excerpt,
    trim_decimal_zeros, values_match_approx,
};
pub use order_book::{
    BOOK_ROW_HEIGHT, BookRowData, aggregate_levels, book_row, book_tick_options,
    clickable_book_row, compute_sigfigs, default_tick_for_price, format_book_size, format_tick,
    nearest_tick_option, nice_step_ceil, placeholder_book_row, sigfig_server_tick, tick_decimals,
    tick_sizes_match, user_order_price_marker, valid_book_tick_size,
};
pub use symbols::{
    category_color, compare_symbol_keys_for_same_ticker, hip3_dex, symbol_icon, symbol_svg_logo,
};
pub use ui::{
    buy_button, ease_out_cubic, label_value, label_value_colored, optional_value_color,
    order_type_button, pane_title, sell_button, signed_number_color, text_color_for_bg,
    text_input_style, timeframe_button, vertical_spacer,
};

#[cfg(test)]
pub(crate) fn assert_close(actual: f64, expected: f64) {
    assert_close_within(actual, expected, 1e-9);
}

#[cfg(test)]
pub(crate) fn assert_close_fine(actual: impl Into<f64>, expected: impl Into<f64>) {
    assert_close_within(actual, expected, 1e-6);
}

#[cfg(test)]
pub(crate) fn assert_close_loose(actual: impl Into<f64>, expected: impl Into<f64>) {
    assert_close_within(actual, expected, 1e-4);
}

#[cfg(test)]
pub(crate) fn assert_close_tight(actual: f64, expected: f64) {
    assert_close_within(actual, expected, 1e-12);
}

#[cfg(test)]
fn assert_close_within(actual: impl Into<f64>, expected: impl Into<f64>, tolerance: f64) {
    let actual = actual.into();
    let expected = expected.into();
    assert!(
        (actual - expected).abs() < tolerance,
        "expected {expected}, got {actual}"
    );
}
