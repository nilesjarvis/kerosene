use super::super::super::AssistantToolCall;
use super::symbols::resolve_valid_symbol;
use crate::account::fetch_all_mids;
use crate::api::{OrderBook, fetch_candles, fetch_exchange_symbols, fetch_order_book};
use crate::assistant::backtest::days_to_ms;
use crate::assistant::summaries::render_candle_summary;

use chrono::Utc;

// ---------------------------------------------------------------------------
// Market Data Tools
// ---------------------------------------------------------------------------

pub(super) async fn execute_market_tool(
    tool_call: &AssistantToolCall,
) -> Option<Result<String, String>> {
    match tool_call {
        AssistantToolCall::Candles {
            symbol,
            interval,
            lookback_days,
        } => {
            let symbol = match resolve_valid_symbol(symbol.clone()).await {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            let now_ms = Utc::now().timestamp_millis() as u64;
            let start_ms = now_ms.saturating_sub(days_to_ms(*lookback_days));
            Some(
                fetch_candles(symbol.clone(), interval.clone(), start_ms, now_ms)
                    .await
                    .map(|candles| render_candle_summary(&symbol, interval, &candles)),
            )
        }
        AssistantToolCall::OrderBook { symbol } => {
            let symbol = match resolve_valid_symbol(symbol.clone()).await {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            let book = match fetch_order_book(symbol.clone(), (None, None)).await {
                Ok(book) => book,
                Err(error) => return Some(Err(error)),
            };
            Some(Ok(render_order_book_summary(&symbol, &book)))
        }
        AssistantToolCall::Symbols => Some(fetch_exchange_symbols().await.map(|symbols| {
            let preview: Vec<String> = symbols
                .iter()
                .take(10)
                .map(|symbol| format!("{} ({})", symbol.key, symbol.category))
                .collect();
            format!(
                "Symbols loaded: {}\nFirst 10: {}",
                symbols.len(),
                preview.join(", ")
            )
        })),
        AssistantToolCall::AllMids { dex } => Some(fetch_all_mids(dex.clone()).await.map(|mids| {
            let mut pairs: Vec<_> = mids.into_iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            let preview: Vec<String> = pairs
                .into_iter()
                .take(10)
                .map(|(key, value)| format!("{key}={value}"))
                .collect();
            format!("allMids dex={dex}\nSample: {}", preview.join(", "))
        })),
        _ => None,
    }
}

fn render_order_book_summary(symbol: &str, book: &OrderBook) -> String {
    let best_bid = book.bids.first().and_then(|bid| positive_price(bid.px));
    let best_ask = book.asks.first().and_then(|ask| positive_price(ask.px));
    let spread = best_bid
        .zip(best_ask)
        .map(|(bid, ask)| ask - bid)
        .filter(|spread| spread.is_finite() && *spread >= 0.0);
    format!(
        "Order book {}\nBest bid: {}\nBest ask: {}\nSpread: {}\nBid levels: {}\nAsk levels: {}",
        symbol,
        optional_price(best_bid),
        optional_price(best_ask),
        optional_price(spread),
        book.bids.len(),
        book.asks.len()
    )
}

fn positive_price(price: f64) -> Option<f64> {
    (price.is_finite() && price > 0.0).then_some(price)
}

fn optional_price(price: Option<f64>) -> String {
    price
        .map(|price| format!("{price:.6}"))
        .unwrap_or_else(|| "Unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::render_order_book_summary;
    use crate::api::{BookLevel, OrderBook};

    #[test]
    fn order_book_summary_marks_missing_or_invalid_top_levels_unavailable() {
        let book = OrderBook {
            bids: vec![BookLevel {
                px: f64::NAN,
                sz: 1.0,
            }],
            asks: Vec::new(),
        };

        let summary = render_order_book_summary("BTC", &book);

        assert!(summary.contains("Best bid: Unavailable"));
        assert!(summary.contains("Best ask: Unavailable"));
        assert!(summary.contains("Spread: Unavailable"));
    }

    #[test]
    fn order_book_summary_reports_positive_finite_spread() {
        let book = OrderBook {
            bids: vec![BookLevel { px: 100.0, sz: 1.0 }],
            asks: vec![BookLevel { px: 101.5, sz: 1.0 }],
        };

        let summary = render_order_book_summary("BTC", &book);

        assert!(summary.contains("Best bid: 100.000000"));
        assert!(summary.contains("Best ask: 101.500000"));
        assert!(summary.contains("Spread: 1.500000"));
    }
}
