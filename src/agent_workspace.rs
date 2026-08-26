use crate::app_state::TradingTerminal;
use crate::chart_indicator::ChartIndicatorId;
use crate::chart_state::ChartId;
use crate::message::Message;

use iced::Task;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const HOST_ACTION_VERSION: u32 = 1;
const MAX_HOST_ACTION_BYTES: usize = 32 * 1024;
const MAX_TARGET_CHARTS: usize = 32;
const MAX_INDICATOR_CHANGES: usize = 32;
const MAX_INDICATOR_APPLICATIONS: usize = MAX_TARGET_CHARTS * MAX_INDICATOR_CHANGES;
pub(crate) const HOST_ACTION_RPC_TITLE: &str = "KEROSENE_HOST_ACTION_V1";

// ---------------------------------------------------------------------------
// Assistant Workspace Action Contract
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentHostActionRequest {
    version: u32,
    tool_call_id: String,
    action: AgentWorkspaceAction,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum AgentWorkspaceAction {
    SetChartIndicators {
        chart_ids: Vec<ChartId>,
        changes: Vec<ChartIndicatorChange>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChartIndicatorChange {
    indicator_id: ChartIndicatorId,
    enabled: bool,
}

#[derive(Serialize)]
struct AgentHostActionResponse {
    success: bool,
    action: &'static str,
    charts: Vec<ChartIndicatorChartResult>,
    persistence_scheduled: bool,
    warnings: Vec<String>,
    error: Option<AgentHostActionError>,
}

#[derive(Serialize)]
struct AgentHostActionError {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct ChartIndicatorChartResult {
    chart_id: ChartId,
    symbol: String,
    display_symbol: String,
    timeframe: String,
    changes: Vec<ChartIndicatorChangeResult>,
}

#[derive(Serialize)]
struct ChartIndicatorChangeResult {
    indicator_id: &'static str,
    label: &'static str,
    previous_enabled: bool,
    enabled: bool,
    outcome: &'static str,
}

impl AgentHostActionResponse {
    fn failure(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            action: "set_chart_indicators",
            charts: Vec::new(),
            persistence_scheduled: false,
            warnings: Vec::new(),
            error: Some(AgentHostActionError {
                code,
                message: message.into(),
            }),
        }
    }

    fn into_json(self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| {
            r#"{"success":false,"action":"set_chart_indicators","charts":[],"persistence_scheduled":false,"warnings":[],"error":{"code":"response_serialization_failed","message":"Kerosene could not serialize the workspace action result"}}"#.to_string()
        })
    }
}

impl TradingTerminal {
    pub(crate) fn handle_agent_host_action(&mut self, payload: &str) -> (String, Task<Message>) {
        if payload.len() > MAX_HOST_ACTION_BYTES {
            return (
                AgentHostActionResponse::failure(
                    "request_too_large",
                    "The workspace action request exceeded Kerosene's size limit",
                )
                .into_json(),
                Task::none(),
            );
        }

        let request = match serde_json::from_str::<AgentHostActionRequest>(payload) {
            Ok(request) => request,
            Err(_) => {
                return (
                    AgentHostActionResponse::failure(
                        "invalid_request",
                        "The workspace action request did not match the supported contract",
                    )
                    .into_json(),
                    Task::none(),
                );
            }
        };

        if request.version != HOST_ACTION_VERSION {
            return (
                AgentHostActionResponse::failure(
                    "unsupported_version",
                    "The workspace action contract version is not supported",
                )
                .into_json(),
                Task::none(),
            );
        }
        if request.tool_call_id.is_empty() || request.tool_call_id.len() > 256 {
            return (
                AgentHostActionResponse::failure(
                    "invalid_tool_call",
                    "The workspace action is missing a valid tool-call identifier",
                )
                .into_json(),
                Task::none(),
            );
        }
        if !self.agent.workspace_actions_allowed
            || !self
                .agent
                .has_running_tool_call(&request.tool_call_id, "kerosene_set_chart_indicators")
        {
            return (
                AgentHostActionResponse::failure(
                    "inactive_tool_call",
                    "The workspace action no longer belongs to the active Assistant turn",
                )
                .into_json(),
                Task::none(),
            );
        }

        match request.action {
            AgentWorkspaceAction::SetChartIndicators { chart_ids, changes } => {
                self.apply_agent_chart_indicator_changes(chart_ids, changes)
            }
        }
    }

    fn apply_agent_chart_indicator_changes(
        &mut self,
        chart_ids: Vec<ChartId>,
        changes: Vec<ChartIndicatorChange>,
    ) -> (String, Task<Message>) {
        if chart_ids.is_empty() || chart_ids.len() > MAX_TARGET_CHARTS {
            return failure_result(
                "invalid_chart_count",
                format!("Choose between 1 and {MAX_TARGET_CHARTS} open charts"),
            );
        }
        if changes.is_empty() || changes.len() > MAX_INDICATOR_CHANGES {
            return failure_result(
                "invalid_change_count",
                format!("Choose between 1 and {MAX_INDICATOR_CHANGES} indicator changes"),
            );
        }
        if chart_ids.len().saturating_mul(changes.len()) > MAX_INDICATOR_APPLICATIONS {
            return failure_result(
                "batch_too_large",
                format!(
                    "One workspace action may apply at most {MAX_INDICATOR_APPLICATIONS} chart-indicator changes"
                ),
            );
        }

        let unique_charts = chart_ids.iter().copied().collect::<HashSet<_>>();
        if unique_charts.len() != chart_ids.len() {
            return failure_result("duplicate_chart", "Each target chart may appear only once");
        }
        let unique_indicators = changes
            .iter()
            .map(|change| change.indicator_id)
            .collect::<HashSet<_>>();
        if unique_indicators.len() != changes.len() {
            return failure_result(
                "duplicate_indicator",
                "Each indicator may appear only once in a workspace action",
            );
        }
        if let Some(indicator) = changes.iter().find_map(|change| {
            (!ChartIndicatorId::ASSISTANT_VISIBLE.contains(&change.indicator_id))
                .then_some(change.indicator_id)
        }) {
            return failure_result(
                "unsupported_indicator",
                format!("{} is not available to the Assistant", indicator.label()),
            );
        }
        if let Some(chart_id) = chart_ids
            .iter()
            .find(|chart_id| !self.charts.contains_key(chart_id))
        {
            return failure_result(
                "chart_not_found",
                format!("Chart {chart_id} is no longer open"),
            );
        }

        let funding_needed = changes.iter().any(|change| {
            change.indicator_id.requires_hydromancer()
                && change.enabled
                && chart_ids.iter().any(|chart_id| {
                    self.charts
                        .get(chart_id)
                        .is_some_and(|instance| !change.indicator_id.is_enabled(instance))
                })
        });
        if funding_needed && self.hydromancer_api_key.trim().is_empty() {
            return failure_result(
                "dependency_missing",
                "Funding rate requires a Hydromancer API key in Settings > Integrations",
            );
        }

        let mut chart_results = Vec::with_capacity(chart_ids.len());
        let mut funding_fetch_ids = Vec::new();
        let mut changed_any = false;

        for chart_id in chart_ids {
            let Some(instance) = self.charts.get_mut(&chart_id) else {
                return failure_result(
                    "chart_not_found",
                    format!("Chart {chart_id} is no longer open"),
                );
            };
            let symbol = instance.symbol.clone();
            let display_symbol = instance.symbol_display.clone();
            let timeframe = instance.interval.label().to_string();
            let mut change_results = Vec::with_capacity(changes.len());

            for change in &changes {
                let previous_enabled = change.indicator_id.is_enabled(instance);
                let changed = change.indicator_id.set_enabled(instance, change.enabled);
                changed_any |= changed;

                if change.indicator_id == ChartIndicatorId::FundingRate && changed {
                    if change.enabled {
                        funding_fetch_ids.push(chart_id);
                    } else {
                        Self::clear_funding_display(instance);
                    }
                }

                change_results.push(ChartIndicatorChangeResult {
                    indicator_id: change.indicator_id.key(),
                    label: change.indicator_id.label(),
                    previous_enabled,
                    enabled: change.enabled,
                    outcome: if changed { "changed" } else { "already_set" },
                });
            }

            instance.chart.macro_indicators = instance.macro_indicators.clone();
            instance.chart.candle_cache.clear();
            chart_results.push(ChartIndicatorChartResult {
                chart_id,
                symbol,
                display_symbol,
                timeframe,
                changes: change_results,
            });
        }

        let persistence_scheduled = if changed_any {
            self.persist_config();
            self.config_save_due_at.is_some()
                && !self.secret_migration_save_blocked
                && !self.config_clear_requested
                && !self.config_cleared_this_session
        } else {
            false
        };
        let warnings = if changed_any && !persistence_scheduled {
            vec![
                "Indicator changes are active for this session, but configuration persistence is paused"
                    .to_string(),
            ]
        } else {
            Vec::new()
        };
        let tasks = funding_fetch_ids
            .into_iter()
            .map(|chart_id| self.maybe_fetch_chart_funding(chart_id))
            .collect::<Vec<_>>();

        (
            AgentHostActionResponse {
                success: true,
                action: "set_chart_indicators",
                charts: chart_results,
                persistence_scheduled,
                warnings,
                error: None,
            }
            .into_json(),
            Task::batch(tasks),
        )
    }
}

fn failure_result(code: &'static str, message: impl Into<String>) -> (String, Task<Message>) {
    (
        AgentHostActionResponse::failure(code, message).into_json(),
        Task::none(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_state::AgentChatEntry;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    fn terminal_with_running_tool() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));
        terminal.agent.entries.push(AgentChatEntry::Tool {
            call_id: "call-1".to_string(),
            name: "kerosene_set_chart_indicators".to_string(),
            detail: None,
            finished: false,
            is_error: false,
            expanded: true,
        });
        terminal.agent.workspace_actions_allowed = true;
        terminal
    }

    fn action_payload(enabled: bool) -> String {
        serde_json::json!({
            "version": HOST_ACTION_VERSION,
            "tool_call_id": "call-1",
            "action": {
                "type": "set_chart_indicators",
                "chart_ids": [7],
                "changes": [{ "indicator_id": "tf_ema_50", "enabled": enabled }],
            }
        })
        .to_string()
    }

    #[test]
    fn assistant_indicator_action_is_idempotent() {
        let mut terminal = terminal_with_running_tool();
        let (first, _task) = terminal.handle_agent_host_action(&action_payload(true));
        let (second, _task) = terminal.handle_agent_host_action(&action_payload(true));
        let first: serde_json::Value = serde_json::from_str(&first).expect("first result");
        let second: serde_json::Value = serde_json::from_str(&second).expect("second result");

        assert_eq!(first["success"], true);
        assert_eq!(first["charts"][0]["changes"][0]["outcome"], "changed");
        assert_eq!(second["success"], true);
        assert_eq!(second["charts"][0]["changes"][0]["outcome"], "already_set");
        assert!(terminal.charts[&7].macro_indicators.tf_ema_50);
    }

    #[test]
    fn quick_trade_is_not_available_to_the_assistant() {
        let mut terminal = terminal_with_running_tool();
        let payload = serde_json::json!({
            "version": HOST_ACTION_VERSION,
            "tool_call_id": "call-1",
            "action": {
                "type": "set_chart_indicators",
                "chart_ids": [7],
                "changes": [{ "indicator_id": "quick_trade", "enabled": true }],
            }
        })
        .to_string();

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");
        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "unsupported_indicator");
        assert!(!terminal.charts[&7].macro_indicators.show_quick_trade);
    }

    #[test]
    fn a_failed_dependency_preflight_does_not_apply_part_of_the_batch() {
        let mut terminal = terminal_with_running_tool();
        terminal.hydromancer_api_key = String::new().into();
        let payload = serde_json::json!({
            "version": HOST_ACTION_VERSION,
            "tool_call_id": "call-1",
            "action": {
                "type": "set_chart_indicators",
                "chart_ids": [7],
                "changes": [
                    { "indicator_id": "tf_ema_50", "enabled": true },
                    { "indicator_id": "funding_rate", "enabled": true },
                ],
            }
        })
        .to_string();

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "dependency_missing");
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
        assert!(!terminal.charts[&7].macro_indicators.show_funding_rate);
    }

    #[test]
    fn stale_tool_call_cannot_mutate_a_chart() {
        let mut terminal = terminal_with_running_tool();
        let payload = action_payload(true).replace("call-1", "stale-call");
        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "inactive_tool_call");
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
    }

    #[test]
    fn aborted_turn_cannot_mutate_a_chart() {
        let mut terminal = terminal_with_running_tool();
        let _task = terminal.update_agent(Message::AgentAbort);
        let (result, _task) = terminal.handle_agent_host_action(&action_payload(true));
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "inactive_tool_call");
        assert!(
            !terminal
                .agent
                .has_running_tool_call("call-1", "kerosene_set_chart_indicators")
        );
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
    }

    #[test]
    fn unknown_action_fields_are_rejected() {
        let mut terminal = terminal_with_running_tool();
        let payload =
            action_payload(true).replace("\"changes\":[", "\"unexpected\":true,\"changes\":[");
        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "invalid_request");
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
    }
}
