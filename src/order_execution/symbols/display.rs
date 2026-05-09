use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;

impl TradingTerminal {
    pub(crate) fn display_name_for_symbol(&self, coin: &str) -> String {
        self.exchange_symbols
            .iter()
            .find(|s| s.key == coin)
            .map(Self::exchange_symbol_display_name)
            .unwrap_or_else(|| coin.split(':').nth(1).unwrap_or(coin).to_string())
    }

    pub(crate) fn exchange_symbol_display_name(sym: &ExchangeSymbol) -> String {
        if let Some(info) = &sym.outcome {
            return format!("{} - {}", Self::outcome_market_label(info), info.side_name);
        }
        sym.display_name
            .clone()
            .unwrap_or_else(|| sym.ticker.clone())
    }

    pub(crate) fn display_size_for_symbol(&self, coin: &str, size: f64) -> String {
        if self.is_outcome_coin(coin) {
            format!("{:.0}", size)
        } else {
            format!("{size:.4}")
        }
    }

    pub(crate) fn display_coin_for_journal(&self, coin: &str) -> String {
        if coin.starts_with('@') || coin.starts_with('#') {
            self.display_name_for_symbol(coin)
        } else {
            coin.to_string()
        }
    }
}
