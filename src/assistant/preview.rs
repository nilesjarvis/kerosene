use crate::assistant::AssistantToolCall;

// ---------------------------------------------------------------------------
// Tool Preview Text
// ---------------------------------------------------------------------------

pub fn preview_tool_call(tool: &AssistantToolCall) -> String {
    match tool {
        AssistantToolCall::DrawdownDca {
            symbol,
            interval,
            lookback_days,
            tranche_usd,
            drawdown_pct,
        } => format!(
            "API request: candleSnapshot coin={symbol}, interval={interval}, lookback_days={lookback_days}, strategy=drawdown_dca, tranche_usd={tranche_usd:.2}, drawdown_pct={drawdown_pct:.2}"
        ),
        AssistantToolCall::HourlyDca {
            symbol,
            lookback_days,
            tranche_usd,
        } => format!(
            "API request: candleSnapshot coin={symbol}, interval=1h, lookback_days={lookback_days}, strategy=hourly_dca, tranche_usd={tranche_usd:.2}"
        ),
        AssistantToolCall::PriceLookup { symbol, interval } => {
            format!("API request: candleSnapshot coin={symbol}, interval={interval}, latest")
        }
        AssistantToolCall::Candles {
            symbol,
            interval,
            lookback_days,
        } => format!(
            "API request: candleSnapshot coin={symbol}, interval={interval}, lookback_days={lookback_days}"
        ),
        AssistantToolCall::OrderBook { symbol } => {
            format!("API request: l2Book coin={symbol}")
        }
        AssistantToolCall::Symbols => {
            "API request: allPerpMetas + perpConciseAnnotations + perpDexs + spotMeta".to_string()
        }
        AssistantToolCall::AllMids { dex } => format!("API request: allMids dex={dex}"),
        AssistantToolCall::AccountSnapshot { address } => {
            format!("API request: account snapshot bundle for user={address}")
        }
        AssistantToolCall::AccountBalance { address } => {
            format!("API request: wallet balance snapshot for user={address}")
        }
        AssistantToolCall::PortfolioHistory { address } => {
            format!("API request: portfolio user={address}")
        }
        AssistantToolCall::IncomeSnapshot { address } => {
            format!("API request: income bundle user={address}")
        }
        AssistantToolCall::LiquidationLevels {
            symbol,
            min_price,
            max_price,
            ..
        } => format!(
            "API request: HyperDash currentLiquidationLevel coin={symbol}, min={min_price:.4}, max={max_price:.4}"
        ),
        AssistantToolCall::LiquidationHeatmap {
            symbol,
            min_price,
            max_price,
            start_time,
            end_time,
            ..
        } => format!(
            "API request: HyperDash liquidationLevels coin={symbol}, min={min_price:.4}, max={max_price:.4}, start={start_time}, end={end_time}"
        ),
        AssistantToolCall::None => "No tool call selected".to_string(),
    }
}
