use crate::api::{ExchangeSymbol, OutcomeSymbolInfo};
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
mod tests {
    use super::format_position_size;

    #[test]
    fn position_size_formatter_hides_zero_fraction() {
        assert_eq!(format_position_size(1.0), "1");
        assert_eq!(format_position_size(25.0), "25");
    }

    #[test]
    fn position_size_formatter_keeps_nonzero_fraction_precision() {
        assert_eq!(format_position_size(1.25), "1.2500");
        assert_eq!(format_position_size(0.125), "0.1250");
    }
}
