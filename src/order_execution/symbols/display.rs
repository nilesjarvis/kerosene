use crate::api::{ExchangeSymbol, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;

impl TradingTerminal {
    pub(crate) fn display_name_for_symbol(&self, coin: &str) -> String {
        if let Some(symbol) = self.exchange_symbols.iter().find(|s| s.key == coin) {
            return Self::exchange_symbol_display_name(symbol);
        }
        if let Some(label) = self.cached_outcome_display_label(coin) {
            return label;
        }
        coin.split(':').nth(1).unwrap_or(coin).to_string()
    }

    /// Cached labels keep expired or not-yet-loaded outcome markets readable;
    /// balance coins ("+NNN") resolve through their trade-coin ("#NNN") entry.
    fn cached_outcome_display_label(&self, coin: &str) -> Option<String> {
        if let Some(label) = self.outcome_display_labels.get(coin) {
            return Some(label.clone());
        }
        let trade_coin = Self::outcome_balance_coin_to_trade_coin(coin)?;
        self.outcome_display_labels.get(&trade_coin).cloned()
    }

    pub(crate) fn exchange_symbol_display_name(sym: &ExchangeSymbol) -> String {
        if let Some(info) = &sym.outcome {
            return outcome_display_label(info);
        }
        sym.display_name
            .clone()
            .unwrap_or_else(|| sym.ticker.clone())
    }

    pub(crate) fn display_size_for_symbol(&self, coin: &str, size: f64) -> String {
        if self.is_outcome_coin(coin) {
            format!("{:.0}", size)
        } else {
            format_position_size(size)
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

fn outcome_display_label(info: &OutcomeSymbolInfo) -> String {
    info.display_label()
}

fn format_position_size(size: f64) -> String {
    let formatted = format!("{size:.4}");
    if let Some((whole, fraction)) = formatted.split_once('.')
        && fraction.chars().all(|ch| ch == '0')
    {
        return whole.to_string();
    }

    formatted
}

#[cfg(test)]
mod tests;
