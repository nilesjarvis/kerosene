use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use std::collections::BTreeMap;

mod card;
mod search;

use search::outcome_symbol_matches_search;

pub(in crate::market_views::outcomes) struct OutcomeMarketSet<'a> {
    pub(in crate::market_views::outcomes) key: String,
    pub(in crate::market_views::outcomes) title: String,
    pub(in crate::market_views::outcomes) quote_symbol: &'a str,
    pub(in crate::market_views::outcomes) is_question_group: bool,
    pub(in crate::market_views::outcomes) outcomes: BTreeMap<u32, Vec<&'a ExchangeSymbol>>,
    pub(in crate::market_views::outcomes) outcome_count: usize,
    pub(in crate::market_views::outcomes) trade_coin_count: usize,
}

impl TradingTerminal {
    pub(super) fn grouped_outcome_markets(&self) -> Vec<OutcomeMarketSet<'_>> {
        let mut grouped: BTreeMap<(u8, u32), OutcomeMarketSet<'_>> = BTreeMap::new();
        let now_ms = self.status_bar_now_ms;
        let query = self.outcome_search_query.trim().to_ascii_lowercase();
        for sym in self.exchange_symbols.iter().filter(|sym| {
            sym.market_type == MarketType::Outcome
                && sym.is_user_selectable_market()
                && !self.exchange_symbol_is_hidden(sym)
                && self.outcome_venue_filter.as_deref().is_none_or(|venue| {
                    sym.outcome.as_ref().and_then(|info| info.venue.as_deref()) == Some(venue)
                })
                && outcome_symbol_matches_search(sym, &query)
        }) {
            if let Some(info) = &sym.outcome {
                let sort_key = match info.question_id {
                    Some(question_id) => (0, question_id),
                    None => (1, info.outcome_id),
                };
                let entry = grouped.entry(sort_key).or_insert_with(|| {
                    let (key, title, is_question_group) = match info.question_id {
                        Some(question_id) => (
                            format!("question:{question_id}"),
                            outcome_question_title(info, now_ms),
                            true,
                        ),
                        None => (
                            format!("outcome:{}", info.outcome_id),
                            info.market_label_with_countdown(now_ms),
                            false,
                        ),
                    };
                    OutcomeMarketSet {
                        key,
                        title,
                        quote_symbol: &info.quote_symbol,
                        is_question_group,
                        outcomes: BTreeMap::new(),
                        outcome_count: 0,
                        trade_coin_count: 0,
                    }
                });
                entry.outcomes.entry(info.outcome_id).or_default().push(sym);
            }
        }

        grouped
            .into_values()
            .map(|mut group| {
                group.outcome_count = group.outcomes.len();
                group.trade_coin_count = group.outcomes.values().map(Vec::len).sum();
                group
            })
            .collect()
    }
}

fn outcome_question_title(info: &crate::api::OutcomeSymbolInfo, now_ms: u64) -> String {
    if info.question_class.as_deref() == Some("priceBucket")
        && let Some(underlying) = info.question_underlying.as_deref()
    {
        return format!("{underlying} price buckets");
    }

    info.question_name
        .as_ref()
        .filter(|name| !name.trim().is_empty() && name.trim() != "Recurring")
        .cloned()
        .unwrap_or_else(|| info.market_label_with_countdown(now_ms))
}

#[cfg(test)]
mod tests;
