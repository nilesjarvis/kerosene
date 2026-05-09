use super::ollama::{OllamaChatMessage, chat_once};
use super::{AssistantPlannedTurn, AssistantToolCall, AssistantTurnInput};

mod model;
mod parsing;
mod prompt;
mod tool_call;

use model::AgentPlan;
use parsing::{extract_ticker_mentions, infer_strategy, resolve_symbol, sanitize_interval};
use prompt::{build_context_text, render_plan};
use tool_call::plan_to_tool_call;

pub use parsing::is_simple_price_query;

pub async fn plan_turn(input: AssistantTurnInput) -> Result<AssistantPlannedTurn, String> {
    let mentions = extract_ticker_mentions(&input.user_prompt);
    let strategy_hint = infer_strategy(&input.user_prompt);
    if is_simple_price_query(&input.user_prompt) {
        let symbol = resolve_symbol(&mentions, &input.context);
        return Ok(AssistantPlannedTurn {
            ollama_url: input.ollama_url,
            model: input.model,
            plan_text:
                "Fast path: detected simple price query; lookup latest price using candleSnapshot."
                    .to_string(),
            tool_call: AssistantToolCall::PriceLookup {
                symbol,
                interval: "1m".to_string(),
            },
            allow_code_execution: input.allow_code_execution,
        });
    }

    let mut plan = AgentPlan {
        objective: input.user_prompt.clone(),
        symbols: if mentions.is_empty() {
            vec![input.context.active_symbol.clone()]
        } else {
            mentions.clone()
        },
        interval: sanitize_interval(&input.context.active_timeframe),
        lookback_days: Some(90),
        strategy: strategy_hint,
        tranche_usd: Some(10_000.0),
        drawdown_pct: Some(10.0),
        assumptions: vec!["Fallback planner used".to_string()],
        steps: vec!["Run a deterministic tool and summarize".to_string()],
        dex: Some("".to_string()),
    };

    if !input.model.trim().is_empty() {
        let planner_prompt = format!(
            "You are a trading analysis planner. Return ONLY valid JSON. \
Schema keys: objective,symbols,interval,lookback_days,strategy,tranche_usd,drawdown_pct,assumptions,steps,dex. \
Allowed strategies: drawdown_dca,hourly_dca,price_lookup,candles,order_book,symbols,all_mids,account_snapshot,account_balance,portfolio_history,income_snapshot,liquidation_levels,liquidation_heatmap. \
Pick exactly one strategy.\nPrompt: {}\nMentions: {:?}\nContext: {}",
            input.user_prompt,
            mentions,
            build_context_text(&input.context, input.use_account_context)
        );
        if let Ok(raw) = chat_once(
            &input.ollama_url,
            &input.model,
            vec![
                OllamaChatMessage {
                    role: "system".to_string(),
                    content: "Plan user query into one deterministic tool call.".to_string(),
                },
                OllamaChatMessage {
                    role: "user".to_string(),
                    content: planner_prompt,
                },
            ],
        )
        .await
            && let Ok(parsed) = serde_json::from_str::<AgentPlan>(&raw)
        {
            plan = parsed;
        }
    }

    let tool_call = plan_to_tool_call(&plan, &input.context, &mentions)?;
    let plan_text = render_plan(&plan);

    Ok(AssistantPlannedTurn {
        ollama_url: input.ollama_url,
        model: input.model,
        plan_text,
        tool_call,
        allow_code_execution: input.allow_code_execution,
    })
}
