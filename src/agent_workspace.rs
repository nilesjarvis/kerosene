use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::Task;
use serde::{Deserialize, Serialize};

mod drawing_model;
mod drawings;
mod indicators;

use drawing_model::ChartDrawingOperation;
pub(crate) use drawing_model::{ASSISTANT_DRAWING_CATALOG, annotation_kind_key};
use drawings::drawing_failure_result;
use indicators::{ChartIndicatorChange, failure_result};

const HOST_ACTION_VERSION: u32 = 1;
const MAX_HOST_ACTION_BYTES: usize = 64 * 1024;
const MAX_TARGET_CHARTS: usize = 32;
const MAX_INDICATOR_CHANGES: usize = 32;
const MAX_DRAWING_OPERATIONS: usize = 64;
const MAX_DRAWING_LABEL_CHARS: usize = 80;
pub(crate) const HOST_ACTION_RPC_TITLE: &str = "KEROSENE_HOST_ACTION_V1";

// ---------------------------------------------------------------------------
// Assistant Workspace Action Admission and Persistence
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
    ManageChartDrawings {
        operations: Vec<ChartDrawingOperation>,
    },
}

impl AgentWorkspaceAction {
    fn tool_name(&self) -> &'static str {
        match self {
            Self::SetChartIndicators { .. } => "kerosene_set_chart_indicators",
            Self::ManageChartDrawings { .. } => "kerosene_manage_chart_drawings",
        }
    }
}

#[derive(Serialize)]
struct AgentHostActionError {
    code: &'static str,
    message: String,
}

impl TradingTerminal {
    pub(crate) fn handle_agent_host_action(&mut self, payload: &str) -> (String, Task<Message>) {
        if payload.len() > MAX_HOST_ACTION_BYTES {
            return failure_result(
                "request_too_large",
                "The workspace action request exceeded Kerosene's size limit",
            );
        }

        let request = match serde_json::from_str::<AgentHostActionRequest>(payload) {
            Ok(request) => request,
            Err(_) => {
                return failure_result(
                    "invalid_request",
                    "The workspace action request did not match the supported contract",
                );
            }
        };
        let tool_name = request.action.tool_name();

        if request.version != HOST_ACTION_VERSION {
            return action_failure_result(
                tool_name,
                "unsupported_version",
                "The workspace action contract version is not supported",
            );
        }
        if request.tool_call_id.is_empty() || request.tool_call_id.len() > 256 {
            return action_failure_result(
                tool_name,
                "invalid_tool_call",
                "The workspace action is missing a valid tool-call identifier",
            );
        }
        if !self.agent.workspace_actions_allowed
            || !self
                .agent
                .has_running_tool_call(&request.tool_call_id, tool_name)
        {
            return action_failure_result(
                tool_name,
                "inactive_tool_call",
                "The workspace action no longer belongs to the active Assistant turn",
            );
        }

        match request.action {
            AgentWorkspaceAction::SetChartIndicators { chart_ids, changes } => {
                self.apply_agent_chart_indicator_changes(chart_ids, changes)
            }
            AgentWorkspaceAction::ManageChartDrawings { operations } => {
                self.apply_agent_chart_drawing_operations(operations)
            }
        }
    }

    fn persist_agent_workspace_changes(&mut self, changed_any: bool) -> bool {
        if !changed_any {
            return false;
        }
        self.persist_config();
        self.config_save_due_at.is_some()
            && !self.secret_migration_save_blocked
            && !self.config_clear_requested
            && !self.config_cleared_this_session
    }
}

fn action_failure_result(
    tool_name: &str,
    code: &'static str,
    message: impl Into<String>,
) -> (String, Task<Message>) {
    if tool_name == "kerosene_manage_chart_drawings" {
        drawing_failure_result(code, message)
    } else {
        failure_result(code, message)
    }
}

#[cfg(test)]
mod tests;
