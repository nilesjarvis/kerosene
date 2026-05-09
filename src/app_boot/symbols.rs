use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::signing::OrderKind;
use std::collections::HashSet;

pub(super) struct BootSymbolSelection {
    pub(super) order_kind: OrderKind,
    pub(super) muted_tickers: HashSet<String>,
    pub(super) active_symbol: String,
    pub(super) active_symbol_display: String,
}

impl TradingTerminal {
    pub(super) fn boot_symbol_selection(cfg: &KeroseneConfig) -> BootSymbolSelection {
        let order_kind = match cfg.order_kind.as_str() {
            "Market" => OrderKind::Market,
            "Chase" => OrderKind::Chase,
            _ => OrderKind::Limit,
        };
        let muted_tickers: HashSet<String> = cfg
            .muted_tickers
            .iter()
            .filter_map(|ticker| Self::normalize_muted_ticker_input(ticker))
            .collect();
        let mut active_symbol = if cfg.active_symbol.is_empty() {
            "HYPE".to_string()
        } else {
            cfg.active_symbol.clone()
        };
        if Self::key_matches_muted_tickers(&[], &muted_tickers, &active_symbol) {
            active_symbol = ["HYPE", "BTC", "ETH"]
                .into_iter()
                .find(|candidate| !Self::key_matches_muted_tickers(&[], &muted_tickers, candidate))
                .unwrap_or("HYPE")
                .to_string();
        }
        let active_symbol_display = active_symbol
            .split(':')
            .nth(1)
            .unwrap_or(&active_symbol)
            .to_string();

        BootSymbolSelection {
            order_kind,
            muted_tickers,
            active_symbol,
            active_symbol_display,
        }
    }
}
