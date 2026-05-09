mod account;
mod backtests;
mod hyperdash;
mod market;
mod symbols;

use super::super::AssistantToolCall;
use account::execute_account_tool;
use backtests::execute_backtest_tool;
use hyperdash::execute_hyperdash_tool;
use market::execute_market_tool;

pub(super) async fn execute_tool_call(tool_call: &AssistantToolCall) -> Result<String, String> {
    if let Some(result) = execute_backtest_tool(tool_call).await {
        return result;
    }
    if let Some(result) = execute_market_tool(tool_call).await {
        return result;
    }
    if let Some(result) = execute_account_tool(tool_call).await {
        return result;
    }
    if let Some(result) = execute_hyperdash_tool(tool_call).await {
        return result;
    }

    Ok("No deterministic tool matched this request. Try asking for price, candles, order book, account, portfolio, income, all mids, symbols, liquidation levels, or liquidation heatmap.".to_string())
}
