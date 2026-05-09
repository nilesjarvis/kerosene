pub(in crate::assistant::planning) fn infer_strategy(prompt: &str) -> String {
    let p = prompt.to_lowercase();
    if is_simple_price_query(prompt) {
        "price_lookup".to_string()
    } else if p.contains("hourly dca") || p.contains("every hour") {
        "hourly_dca".to_string()
    } else if p.contains("drawdown") || p.contains("every time it went down") {
        "drawdown_dca".to_string()
    } else if p.contains("order book") || p.contains("bid") || p.contains("ask") {
        "order_book".to_string()
    } else if p.contains("liquidation heatmap") || p.contains("heatmap") {
        "liquidation_heatmap".to_string()
    } else if p.contains("liquidation") {
        "liquidation_levels".to_string()
    } else if p.contains("income") || p.contains("interest") {
        "income_snapshot".to_string()
    } else if p.contains("account balance")
        || p.contains("current balance")
        || p.contains("equity")
        || p.contains("withdrawable")
    {
        "account_balance".to_string()
    } else if p.contains("portfolio") || p.contains("equity history") {
        "portfolio_history".to_string()
    } else if p.contains("account") || p.contains("open orders") || p.contains("fills") {
        "account_snapshot".to_string()
    } else if p.contains("all mids") || p.contains("mids") {
        "all_mids".to_string()
    } else if p.contains("symbols") || p.contains("ticker list") {
        "symbols".to_string()
    } else {
        "candles".to_string()
    }
}

pub(in crate::assistant::planning) fn force_strategy_from_objective(
    objective: &str,
) -> Option<String> {
    let p = objective.to_lowercase();
    if p.contains("hourly dca") || p.contains("every hour") {
        Some("hourly_dca".to_string())
    } else if p.contains("account balance")
        || p.contains("current balance")
        || p.contains("equity")
        || p.contains("withdrawable")
    {
        Some("account_balance".to_string())
    } else {
        None
    }
}

pub fn is_simple_price_query(prompt: &str) -> bool {
    let p = prompt.to_lowercase();
    p.contains(" price")
        || p.starts_with("price")
        || (p.contains("what is") && p.contains("price"))
        || p.contains("quote")
        || p.contains("trading at")
}
