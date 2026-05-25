use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use std::collections::BTreeMap;

mod card;

pub(in crate::market_views::outcomes) struct OutcomeMarketSet<'a> {
    pub(in crate::market_views::outcomes) key: String,
    pub(in crate::market_views::outcomes) title: String,
    pub(in crate::market_views::outcomes) quote_symbol: String,
    pub(in crate::market_views::outcomes) is_question_group: bool,
    pub(in crate::market_views::outcomes) outcomes: BTreeMap<u32, Vec<&'a ExchangeSymbol>>,
    pub(in crate::market_views::outcomes) outcome_count: usize,
    pub(in crate::market_views::outcomes) trade_coin_count: usize,
}

impl TradingTerminal {
    pub(super) fn grouped_outcome_markets(&self) -> Vec<OutcomeMarketSet<'_>> {
        let mut grouped: BTreeMap<(u8, u32), OutcomeMarketSet<'_>> = BTreeMap::new();
        let query = self.outcome_search_query.trim();
        for sym in self.exchange_symbols.iter().filter(|sym| {
            sym.market_type == MarketType::Outcome
                && sym.is_user_selectable_market()
                && !self.exchange_symbol_is_hidden(sym)
                && outcome_symbol_matches_search(sym, query)
        }) {
            if let Some(info) = &sym.outcome {
                let (sort_key, key, title, is_question_group) = match info.question_id {
                    Some(question_id) => (
                        (0, question_id),
                        format!("question:{question_id}"),
                        outcome_question_title(info),
                        true,
                    ),
                    None => (
                        (1, info.outcome_id),
                        format!("outcome:{}", info.outcome_id),
                        info.market_label_with_countdown(Self::now_ms()),
                        false,
                    ),
                };
                let entry = grouped.entry(sort_key).or_insert_with(|| OutcomeMarketSet {
                    key,
                    title,
                    quote_symbol: info.quote_symbol.clone(),
                    is_question_group,
                    outcomes: BTreeMap::new(),
                    outcome_count: 0,
                    trade_coin_count: 0,
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

fn outcome_question_title(info: &crate::api::OutcomeSymbolInfo) -> String {
    if info.question_class.as_deref() == Some("priceBucket")
        && let Some(underlying) = info.question_underlying.as_deref()
    {
        return format!("{underlying} price buckets");
    }

    info.question_name
        .as_ref()
        .filter(|name| !name.trim().is_empty() && name.trim() != "Recurring")
        .cloned()
        .unwrap_or_else(|| info.market_label_with_countdown(TradingTerminal::now_ms()))
}

fn outcome_symbol_matches_search(symbol: &ExchangeSymbol, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let haystack = outcome_symbol_search_haystack(symbol).to_ascii_lowercase();
    query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .all(|term| haystack.contains(&term))
}

fn outcome_symbol_search_haystack(symbol: &ExchangeSymbol) -> String {
    let mut values = String::new();
    push_search_value(&mut values, symbol.key.as_str());
    push_search_value(&mut values, symbol.ticker.as_str());
    push_search_value(&mut values, symbol.category.as_str());
    if let Some(display_name) = symbol.display_name.as_deref() {
        push_search_value(&mut values, display_name);
    }
    for keyword in &symbol.keywords {
        push_search_value(&mut values, keyword);
    }

    let Some(info) = symbol.outcome.as_ref() else {
        return values;
    };

    let owned = [
        info.market_label(),
        info.display_label(),
        info.side_condition_label(),
        info.outcome_id.to_string(),
        info.question_id
            .map(|question_id| question_id.to_string())
            .unwrap_or_default(),
        info.bucket_index
            .map(|bucket_index| bucket_index.to_string())
            .unwrap_or_default(),
    ];
    for value in owned {
        push_search_value(&mut values, &value);
    }

    for value in [
        info.question_name.as_deref(),
        info.question_description.as_deref(),
        info.question_class.as_deref(),
        info.question_underlying.as_deref(),
        info.question_expiry.as_deref(),
        info.question_period.as_deref(),
        Some(info.side_name.as_str()),
        Some(info.outcome_name.as_str()),
        Some(info.description.as_str()),
        info.class.as_deref(),
        info.underlying.as_deref(),
        info.expiry.as_deref(),
        info.target_price.as_deref(),
        info.period.as_deref(),
        Some(info.quote_symbol.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        push_search_value(&mut values, value);
    }
    for threshold in &info.question_price_thresholds {
        push_search_value(&mut values, threshold);
    }

    values
}

fn push_search_value(haystack: &mut String, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    if !haystack.is_empty() {
        haystack.push(' ');
    }
    haystack.push_str(value);
}

#[cfg(test)]
mod tests;
