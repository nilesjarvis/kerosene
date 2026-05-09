use super::super::AssistantRuntimeContext;
use super::model::AgentPlan;
use super::parsing::sanitize_interval;

pub(super) fn build_context_text(ctx: &AssistantRuntimeContext, include_account: bool) -> String {
    let price = ctx
        .latest_price
        .map(|v| format!("{v:.6}"))
        .unwrap_or_else(|| "n/a".to_string());
    let account = if include_account {
        ctx.account_summary
            .clone()
            .unwrap_or_else(|| "none".to_string())
    } else {
        "disabled".to_string()
    };
    format!(
        "active_symbol={}; active_timeframe={}; latest_price={}; account_summary={}; wallet_connected={}; has_hyperdash_key={}",
        ctx.active_symbol,
        ctx.active_timeframe,
        price,
        account,
        ctx.connected_address.is_some(),
        ctx.hyperdash_api_key.is_some()
    )
}

pub(super) fn render_plan(plan: &AgentPlan) -> String {
    let strategy = plan.strategy.trim().to_lowercase();
    let mut lines = vec![
        format!("Objective: {}", plan.objective),
        format!("Strategy: {}", plan.strategy),
        format!(
            "Symbols: {}",
            if plan.symbols.is_empty() {
                "n/a".to_string()
            } else {
                plan.symbols.join(", ")
            }
        ),
    ];
    if matches!(
        strategy.as_str(),
        "drawdown_dca" | "hourly_dca" | "candles" | "price_lookup"
    ) {
        lines.push(format!("Interval: {}", sanitize_interval(&plan.interval)));
    }
    if matches!(strategy.as_str(), "drawdown_dca" | "hourly_dca" | "candles")
        && let Some(days) = plan.lookback_days
    {
        lines.push(format!("Lookback: {days} days"));
    }
    if matches!(strategy.as_str(), "drawdown_dca" | "hourly_dca")
        && let Some(v) = plan.tranche_usd
    {
        lines.push(format!("Tranche: ${v:.2}"));
    }
    if strategy == "drawdown_dca"
        && let Some(v) = plan.drawdown_pct
    {
        lines.push(format!("Drawdown: {v:.2}%"));
    }
    lines.join("\n")
}
