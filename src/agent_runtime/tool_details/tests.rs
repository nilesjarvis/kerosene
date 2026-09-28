use super::*;
use serde_json::json;

#[test]
fn tool_call_detail_compacts_long_symbol_lists() {
    let args = json!({
        "symbols": ["BTC", "ETH", "SOL", "HYPE", "DOGE"]
    });

    assert_eq!(
        tool_call_detail("kerosene_market_data", Some(&args)).as_deref(),
        Some("BTC, ETH, SOL +2 · Current mids and market metadata")
    );
}

#[test]
fn drawing_tool_detail_counts_unique_charts_without_exposing_geometry() {
    let args = json!({
        "operations": [
            { "operation": "add", "chart_id": 7, "drawing": { "type": "horizontal_level", "price": 60_000 } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "vertical_line", "time_ms": 1_700_000_000_000_u64 } },
            { "operation": "remove", "chart_id": 9, "drawing_id": 3 }
        ]
    });

    assert_eq!(
        tool_call_detail("kerosene_manage_chart_drawings", Some(&args)).as_deref(),
        Some("2 charts · 3 drawing operations")
    );
}
