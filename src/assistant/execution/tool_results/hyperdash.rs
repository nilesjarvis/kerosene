use super::super::super::AssistantToolCall;
use super::symbols::resolve_valid_symbol;
use crate::hyperdash_api::{fetch_liquidation_heatmap, fetch_liquidation_levels};

// ---------------------------------------------------------------------------
// HyperDash Tools
// ---------------------------------------------------------------------------

pub(super) async fn execute_hyperdash_tool(
    tool_call: &AssistantToolCall,
) -> Option<Result<String, String>> {
    match tool_call {
        AssistantToolCall::LiquidationLevels {
            symbol,
            min_price,
            max_price,
            api_key,
        } => {
            let symbol = match resolve_valid_symbol(symbol.clone()).await {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            Some(
                fetch_liquidation_levels(symbol, *min_price, *max_price, api_key.clone())
                    .await
                    .map(|liq| {
                        format!(
                            "Current liquidation levels {}\nRange: {:.4} - {:.4}\nEntries: {}\nTotal amount: {:.4}",
                            liq.coin,
                            liq.min,
                            liq.max,
                            liq.liquidations.len(),
                            liq.total_amount
                        )
                    }),
            )
        }
        AssistantToolCall::LiquidationHeatmap {
            symbol,
            min_price,
            max_price,
            start_time,
            end_time,
            api_key,
        } => {
            let symbol = match resolve_valid_symbol(symbol.clone()).await {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            Some(
                fetch_liquidation_heatmap(
                    symbol,
                    *min_price,
                    *max_price,
                    *start_time,
                    *end_time,
                    api_key.clone(),
                )
                .await
                .map(|heat| {
                    format!(
                        "Liquidation heatmap {}\nPrice range: {:.4} - {:.4}\nCells: {}\nMax abs USD: {:.4}",
                        heat.coin,
                        heat.min_price,
                        heat.max_price,
                        heat.rects.len(),
                        heat.max_abs_usd
                    )
                }),
            )
        }
        _ => None,
    }
}
