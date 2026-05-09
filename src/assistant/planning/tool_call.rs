use super::super::{AssistantRuntimeContext, AssistantToolCall};
use super::model::AgentPlan;
use super::parsing::{
    default_liq_range, force_strategy_from_objective, infer_strategy, parse_lookback_days,
    parse_usd_amount, pick_symbol_candidate, sanitize_interval,
};

use chrono::Utc;

pub(super) fn plan_to_tool_call(
    plan: &AgentPlan,
    ctx: &AssistantRuntimeContext,
    mentions: &[String],
) -> Result<AssistantToolCall, String> {
    let symbol = pick_symbol_candidate(&plan.symbols, mentions, ctx);
    let interval = sanitize_interval(&plan.interval);
    let inferred_days = parse_lookback_days(&plan.objective);
    let lookback_days = inferred_days
        .or(plan.lookback_days)
        .unwrap_or(90)
        .clamp(1, 3650);
    let inferred_tranche = parse_usd_amount(&plan.objective);
    let forced_strategy = force_strategy_from_objective(&plan.objective);

    let strategy = forced_strategy.unwrap_or_else(|| plan.strategy.trim().to_lowercase());
    Ok(match strategy.as_str() {
        "hourly_dca" => AssistantToolCall::HourlyDca {
            symbol,
            lookback_days,
            tranche_usd: inferred_tranche
                .or(plan.tranche_usd)
                .unwrap_or(100.0)
                .max(1.0),
        },
        "drawdown_dca" => AssistantToolCall::DrawdownDca {
            symbol,
            interval,
            lookback_days,
            tranche_usd: inferred_tranche
                .or(plan.tranche_usd)
                .unwrap_or(10_000.0)
                .max(1.0),
            drawdown_pct: plan.drawdown_pct.unwrap_or(10.0).clamp(1.0, 90.0),
        },
        "price_lookup" => AssistantToolCall::PriceLookup {
            symbol,
            interval: "1m".to_string(),
        },
        "candles" => AssistantToolCall::Candles {
            symbol,
            interval,
            lookback_days,
        },
        "order_book" => AssistantToolCall::OrderBook { symbol },
        "symbols" => AssistantToolCall::Symbols,
        "all_mids" => AssistantToolCall::AllMids {
            dex: plan.dex.clone().unwrap_or_default(),
        },
        "account_snapshot" => AssistantToolCall::AccountSnapshot {
            address: ctx
                .connected_address
                .clone()
                .ok_or_else(|| "No connected wallet address for account snapshot".to_string())?,
        },
        "account_balance" => AssistantToolCall::AccountBalance {
            address: ctx
                .connected_address
                .clone()
                .ok_or_else(|| "No connected wallet address for account balance".to_string())?,
        },
        "portfolio_history" => AssistantToolCall::PortfolioHistory {
            address: ctx
                .connected_address
                .clone()
                .ok_or_else(|| "No connected wallet address for portfolio history".to_string())?,
        },
        "income_snapshot" => AssistantToolCall::IncomeSnapshot {
            address: ctx
                .connected_address
                .clone()
                .ok_or_else(|| "No connected wallet address for income snapshot".to_string())?,
        },
        "liquidation_levels" => {
            let (min_price, max_price) = default_liq_range(ctx.latest_price);
            AssistantToolCall::LiquidationLevels {
                symbol,
                min_price,
                max_price,
                api_key: ctx
                    .hyperdash_api_key
                    .clone()
                    .ok_or_else(|| "Missing HyperDash API key".to_string())?,
            }
        }
        "liquidation_heatmap" => {
            let now_s = (Utc::now().timestamp_millis() as u64) / 1000;
            let (min_price, max_price) = default_liq_range(ctx.latest_price);
            AssistantToolCall::LiquidationHeatmap {
                symbol,
                min_price,
                max_price,
                start_time: now_s.saturating_sub(30 * 24 * 60 * 60),
                end_time: now_s,
                api_key: ctx
                    .hyperdash_api_key
                    .clone()
                    .ok_or_else(|| "Missing HyperDash API key".to_string())?,
            }
        }
        _ => match infer_strategy(&plan.objective).as_str() {
            "order_book" => AssistantToolCall::OrderBook { symbol },
            "candles" => AssistantToolCall::Candles {
                symbol,
                interval,
                lookback_days,
            },
            _ => AssistantToolCall::None,
        },
    })
}
