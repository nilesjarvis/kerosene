use super::super::super::AssistantToolCall;
use super::symbols::resolve_valid_symbol;
use crate::assistant::backtest::{
    lookup_latest_price, render_drawdown_result, render_hourly_dca_result,
    render_price_lookup_result, run_drawdown_dca_backtest, run_hourly_dca_backtest,
};

// ---------------------------------------------------------------------------
// Backtest And Price Tools
// ---------------------------------------------------------------------------

pub(super) async fn execute_backtest_tool(
    tool_call: &AssistantToolCall,
) -> Option<Result<String, String>> {
    match tool_call {
        AssistantToolCall::HourlyDca {
            symbol,
            lookback_days,
            tranche_usd,
        } => {
            let symbol = match resolve_valid_symbol(symbol.clone()).await {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            Some(
                run_hourly_dca_backtest(symbol, *lookback_days, *tranche_usd)
                    .await
                    .map(render_hourly_dca_result),
            )
        }
        AssistantToolCall::DrawdownDca {
            symbol,
            interval,
            lookback_days,
            tranche_usd,
            drawdown_pct,
        } => {
            let symbol = match resolve_valid_symbol(symbol.clone()).await {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            Some(
                run_drawdown_dca_backtest(
                    symbol,
                    interval.clone(),
                    *lookback_days,
                    *tranche_usd,
                    *drawdown_pct,
                )
                .await
                .map(render_drawdown_result),
            )
        }
        AssistantToolCall::PriceLookup { symbol, interval } => {
            let symbol = match resolve_valid_symbol(symbol.clone()).await {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            Some(
                lookup_latest_price(symbol, interval.clone())
                    .await
                    .map(|result| render_price_lookup_result(&result)),
            )
        }
        _ => None,
    }
}
