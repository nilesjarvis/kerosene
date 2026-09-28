use crate::api::ExchangeSymbol;

/// Matches all terms in a query that has already been ASCII-lowercased.
pub(super) fn outcome_symbol_matches_search(
    symbol: &ExchangeSymbol,
    lowercase_query: &str,
) -> bool {
    if lowercase_query.is_empty() {
        return true;
    }

    let mut haystack = outcome_symbol_search_haystack(symbol);
    haystack.make_ascii_lowercase();
    lowercase_query
        .split_whitespace()
        .all(|term| haystack.contains(term))
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
        info.venue.as_deref(),
        info.venue_label(),
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
