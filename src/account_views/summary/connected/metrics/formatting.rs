use crate::account_views::invalid_account_data;

// ---------------------------------------------------------------------------
// Connected Summary Formatting
// ---------------------------------------------------------------------------

pub(in crate::account_views::summary::connected) fn summary_number_string(
    value: Option<f64>,
) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(invalid_account_data)
}

pub(in crate::account_views::summary::connected) fn summary_percent_string(
    value: Option<f64>,
) -> String {
    value
        .map(|value| format!("{:.2}%", value * 100.0))
        .unwrap_or_else(invalid_account_data)
}

pub(in crate::account_views::summary::connected) fn leverage_string(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}x"))
        .unwrap_or_else(invalid_account_data)
}
