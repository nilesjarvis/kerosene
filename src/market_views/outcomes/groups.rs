use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use std::collections::BTreeMap;

mod card;

impl TradingTerminal {
    pub(super) fn grouped_outcome_symbols(&self) -> BTreeMap<u32, Vec<&ExchangeSymbol>> {
        let mut grouped = BTreeMap::new();
        for sym in self.exchange_symbols.iter().filter(|sym| {
            sym.market_type == MarketType::Outcome && !self.exchange_symbol_is_muted(sym)
        }) {
            if let Some(info) = &sym.outcome {
                grouped
                    .entry(info.outcome_id)
                    .or_insert_with(Vec::new)
                    .push(sym);
            }
        }
        grouped
    }
}
