use serde_json::Value;

// ---------------------------------------------------------------------------
// Assistant Tool Request Summaries
// ---------------------------------------------------------------------------

pub(super) fn tool_call_detail(name: &str, args: Option<&Value>) -> Option<String> {
    let args = args?.as_object()?;
    let field = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(|value| bounded_tool_value(value, 32))
    };
    let symbols = || summarized_symbols(args.get("symbols"));

    let segments = match name {
        "kerosene_data" => vec![field("section").map(|value| title_case(&value))?],
        "kerosene_market_data" => vec![symbols()?, "Current mids and market metadata".to_string()],
        "kerosene_set_chart_indicators" => {
            let chart_count = args
                .get("chart_ids")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            let change_count = args
                .get("changes")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            vec![
                format!(
                    "{chart_count} chart{}",
                    if chart_count == 1 { "" } else { "s" }
                ),
                format!(
                    "{change_count} indicator change{}",
                    if change_count == 1 { "" } else { "s" }
                ),
            ]
        }
        "kerosene_manage_chart_drawings" => {
            let operation_count = args
                .get("operations")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            let chart_count = args
                .get("operations")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|operation| operation.get("chart_id").and_then(Value::as_u64))
                .collect::<std::collections::HashSet<_>>()
                .len();
            vec![
                format!(
                    "{chart_count} chart{}",
                    if chart_count == 1 { "" } else { "s" }
                ),
                format!(
                    "{operation_count} drawing operation{}",
                    if operation_count == 1 { "" } else { "s" }
                ),
            ]
        }
        "kerosene_activity" => {
            let mut segments = Vec::new();
            push_field(&mut segments, field("kind"), title_case);
            push_field(&mut segments, field("mode"), title_case);
            push_optional(&mut segments, field("symbol"));
            if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                segments.push(format!("Up to {limit} rows"));
            }
            segments
        }
        "kerosene_journal" => {
            let mut segments = Vec::new();
            push_field(&mut segments, field("operation"), journal_operation_label);
            push_field(&mut segments, field("metric"), metric_label);
            push_optional(&mut segments, field("symbol"));
            push_field(&mut segments, field("status"), title_case);
            if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                segments.push(format!("Up to {limit} trades"));
            }
            segments
        }
        "kerosene_calculate" => {
            let mut segments = Vec::new();
            push_field(&mut segments, field("operation"), calculation_label);
            push_optional(&mut segments, field("symbol"));
            push_optional(
                &mut segments,
                field("interval").map(|value| format!("{value} candles")),
            );
            if let Some(shock) = args.get("shock_pct").and_then(Value::as_f64) {
                segments.push(format!("{shock:+.1}% shock"));
            }
            segments
        }
        "kerosene_risk" => vec!["Clearinghouse, spot, portfolio, and income scopes".to_string()],
        "kerosene_positioning" => {
            let mut segments = Vec::new();
            push_optional(&mut segments, symbols());
            push_field(&mut segments, field("timeframe"), timeframe_label);
            segments
        }
        "kerosene_pnl_card_match" => {
            let mut segments = Vec::new();
            push_optional(&mut segments, field("symbol"));
            push_optional(&mut segments, field("side").map(|value| title_case(&value)));
            segments
        }
        "kerosene_ohlcv" => {
            let mut segments = Vec::new();
            push_optional(&mut segments, field("symbol"));
            push_optional(
                &mut segments,
                field("interval").map(|value| format!("{value} candles")),
            );
            if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                segments.push(format!("Up to {limit} rows"));
            }
            segments
        }
        "kerosene_sessions" => {
            let mut segments = Vec::new();
            push_optional(&mut segments, field("symbol"));
            if let Some(days) = args.get("lookback_days").and_then(Value::as_u64) {
                segments.push(format!("{days}-day lookback"));
            }
            segments
        }
        _ => Vec::new(),
    };

    (!segments.is_empty()).then(|| segments.join(" · "))
}

fn push_optional(segments: &mut Vec<String>, value: Option<String>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        segments.push(value);
    }
}

fn push_field(
    segments: &mut Vec<String>,
    value: Option<String>,
    label: impl FnOnce(&str) -> String,
) {
    if let Some(value) = value {
        segments.push(label(&value));
    }
}

fn summarized_symbols(value: Option<&Value>) -> Option<String> {
    let symbols = value?.as_array()?;
    let visible = symbols
        .iter()
        .filter_map(Value::as_str)
        .take(3)
        .map(|symbol| bounded_tool_value(symbol, 24))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return None;
    }

    let hidden = symbols.len().saturating_sub(visible.len());
    let mut summary = visible.join(", ");
    if hidden > 0 {
        summary.push_str(&format!(" +{hidden}"));
    }
    Some(summary)
}

fn bounded_tool_value(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    let mut chars = value.chars();
    let mut bounded = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        let _ = bounded.pop();
        bounded.push('…');
    }
    bounded
}

fn title_case(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn journal_operation_label(value: &str) -> String {
    match value {
        "best" => "Best trades".to_string(),
        "worst" => "Worst trades".to_string(),
        "summary" => "Performance summary".to_string(),
        "list" => "Recent trades".to_string(),
        _ => title_case(value),
    }
}

fn metric_label(value: &str) -> String {
    match value {
        "net_pnl" => "Net PnL".to_string(),
        "gross_pnl" => "Gross PnL".to_string(),
        "return_on_entry_pct" => "Return on entry".to_string(),
        "net_pnl_per_volume_pct" => "Net PnL per volume".to_string(),
        _ => title_case(value),
    }
}

fn calculation_label(value: &str) -> String {
    match value {
        "exposure" => "Exposure".to_string(),
        "liquidation_buffers" => "Liquidation buffers".to_string(),
        "stress" => "Stress test".to_string(),
        "fill_aggregation" => "Fill aggregation".to_string(),
        "funding_aggregation" => "Funding aggregation".to_string(),
        "portfolio_reconciliation" => "Portfolio reconciliation".to_string(),
        "market_statistics" => "Market statistics".to_string(),
        _ => title_case(value),
    }
}

fn timeframe_label(value: &str) -> String {
    match value {
        "FIFTEEN_MINUTES" => "15-minute change".to_string(),
        "ONE_HOUR" => "1-hour change".to_string(),
        "FOUR_HOURS" => "4-hour change".to_string(),
        _ => title_case(value),
    }
}

#[cfg(test)]
mod tests;
