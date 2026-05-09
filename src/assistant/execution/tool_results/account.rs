use super::super::super::AssistantToolCall;
use crate::account::{fetch_account_data, fetch_wallet_tracker_snapshot};
use crate::account_analytics::{fetch_income_data, fetch_portfolio_history};
use crate::assistant::summaries::{
    render_account_summary, render_income_summary, render_portfolio_summary,
};

// ---------------------------------------------------------------------------
// Account Data Tools
// ---------------------------------------------------------------------------

pub(super) async fn execute_account_tool(
    tool_call: &AssistantToolCall,
) -> Option<Result<String, String>> {
    match tool_call {
        AssistantToolCall::AccountSnapshot { address } => {
            Some(execute_account_snapshot(address).await)
        }
        AssistantToolCall::AccountBalance { address } => Some(
            fetch_wallet_tracker_snapshot(address.clone())
                .await
                .map(|snap| {
                    format!(
                        "Account balance {}\nEquity: {}\nWithdrawable: {}",
                        address,
                        account_tool_number(snap.equity),
                        account_tool_number(snap.withdrawable)
                    )
                }),
        ),
        AssistantToolCall::PortfolioHistory { address } => Some(
            fetch_portfolio_history(address.clone())
                .await
                .map(|history| render_portfolio_summary(address, &history)),
        ),
        AssistantToolCall::IncomeSnapshot { address } => Some(
            fetch_income_data(address.clone())
                .await
                .map(|income| render_income_summary(address, &income)),
        ),
        _ => None,
    }
}

async fn execute_account_snapshot(address: &str) -> Result<String, String> {
    match fetch_account_data(address.to_string()).await {
        Ok(data) => Ok(render_account_summary(address, &data)),
        Err(error) => {
            if error.contains("clearinghouseState deserialize failed") {
                let snap = fetch_wallet_tracker_snapshot(address.to_string()).await?;
                Ok(format!(
                    "Account snapshot fallback {}\nEquity: {}\nWithdrawable: {}\nNote: detailed account endpoint returned partial/null payload",
                    address,
                    account_tool_number(snap.equity),
                    account_tool_number(snap.withdrawable)
                ))
            } else {
                Err(error)
            }
        }
    }
}

fn account_tool_number(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "Invalid data".to_string())
}
