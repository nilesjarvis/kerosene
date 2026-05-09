use crate::account::AccountData;
use crate::account_analytics::{IncomeSnapshot, PortfolioHistory};
use crate::api::Candle;

// ---------------------------------------------------------------------------
// Tool Result Summaries
// ---------------------------------------------------------------------------

pub(super) fn render_account_summary(address: &str, data: &AccountData) -> String {
    let warning = data
        .completeness
        .warning_summary()
        .map(|warning| format!("\nWarning: {warning}"))
        .unwrap_or_default();
    format!(
        "Account snapshot {}\nPortfolio margin: {}\nPositions: {}\nOpen orders: {}\nFills: {}\nFunding entries: {}\nWithdrawable: {}\nTotal margin used: {}{}",
        address,
        if data.is_portfolio_margin() {
            "yes"
        } else {
            "no"
        },
        data.clearinghouse.asset_positions.len(),
        data.open_orders.len(),
        data.fills.len(),
        data.funding_history.len(),
        account_number_summary(data.withdrawable()),
        account_number_summary(data.total_margin_used()),
        warning,
    )
}

pub(super) fn render_portfolio_summary(address: &str, history: &PortfolioHistory) -> String {
    let keys = history
        .buckets
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let skipped_invalid_points: usize = history
        .buckets
        .values()
        .map(|bucket| bucket.skipped_invalid_points)
        .sum();
    let warning = if skipped_invalid_points > 0 {
        format!("\nWarning: skipped {skipped_invalid_points} invalid portfolio history points")
    } else {
        String::new()
    };
    format!(
        "Portfolio history {}\nBuckets: {}\nNames: {}{}",
        address,
        history.buckets.len(),
        if keys.is_empty() { "none" } else { &keys },
        warning,
    )
}

pub(super) fn render_income_summary(address: &str, income: &IncomeSnapshot) -> String {
    let warning = if income.invalid_token_rows > 0 || income.invalid_interest_rows > 0 {
        format!(
            "\nWarning: skipped {} invalid token rows and {} invalid interest rows",
            income.invalid_token_rows, income.invalid_interest_rows
        )
    } else {
        String::new()
    };
    format!(
        "Income snapshot {}\nTotal earned: {:.4}\n24h: {:.4}\n7d: {:.4}\n30d: {:.4}\nNet yearly projection: {:.4}\nHourly rows: {}{}",
        address,
        income.earned_total,
        income.earned_24h,
        income.earned_7d,
        income.earned_30d,
        income.net_yearly_projection,
        income.recent_hourly_payments.len(),
        warning,
    )
}

pub(super) fn render_candle_summary(symbol: &str, interval: &str, candles: &[Candle]) -> String {
    if candles.is_empty() {
        return format!("No candles returned for {symbol} ({interval})");
    }
    let Some(first) = candles
        .first()
        .map(|c| c.close)
        .filter(|close| close.is_finite())
    else {
        return format!("Invalid candle data returned for {symbol} ({interval})");
    };
    let Some(last) = candles
        .last()
        .map(|c| c.close)
        .filter(|close| close.is_finite())
    else {
        return format!("Invalid candle data returned for {symbol} ({interval})");
    };
    let change = if first.abs() > 1e-12 {
        (last - first) / first * 100.0
    } else {
        0.0
    };
    format!(
        "Candles {} ({interval})\nCount: {}\nFirst close: {:.6}\nLast close: {:.6}\nChange: {:.2}%",
        symbol,
        candles.len(),
        first,
        last,
        change
    )
}

fn account_number_summary(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "Invalid data".to_string())
}

#[cfg(test)]
mod tests {
    use super::render_candle_summary;
    use crate::api::Candle;

    fn candle(open_time: u64, close: f64) -> Candle {
        Candle {
            open_time,
            close_time: open_time + 60_000,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn candle_summary_reports_empty_or_invalid_data_without_zero_prices() {
        assert_eq!(
            render_candle_summary("BTC", "1h", &[]),
            "No candles returned for BTC (1h)"
        );
        assert_eq!(
            render_candle_summary("BTC", "1h", &[candle(1, f64::NAN)]),
            "Invalid candle data returned for BTC (1h)"
        );
    }

    #[test]
    fn candle_summary_uses_first_and_last_finite_closes() {
        let summary = render_candle_summary("BTC", "1h", &[candle(1, 100.0), candle(2, 110.0)]);

        assert!(summary.contains("First close: 100.000000"));
        assert!(summary.contains("Last close: 110.000000"));
        assert!(summary.contains("Change: 10.00%"));
    }
}
