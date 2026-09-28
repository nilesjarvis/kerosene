use super::AgentRuntimeEvent;
use super::tool_details::tool_call_detail;
use serde_json::{Value, json};
use std::io::Write;

// ---------------------------------------------------------------------------
// Pi RPC Commands and Events
// ---------------------------------------------------------------------------

pub(super) fn write_rpc_command(stdin: &mut impl Write, value: &Value) -> std::io::Result<()> {
    serde_json::to_writer(&mut *stdin, value)?;
    stdin.write_all(b"\n")?;
    stdin.flush()
}

pub(super) fn extension_ui_response_value(request_id: String, value: Option<String>) -> Value {
    value.map_or_else(
        || {
            json!({
                "type": "extension_ui_response",
                "id": request_id,
                "cancelled": true,
            })
        },
        |value| {
            json!({
                "type": "extension_ui_response",
                "id": request_id,
                "value": value,
            })
        },
    )
}

pub(super) fn parse_rpc_event(generation: u64, value: &Value) -> Option<AgentRuntimeEvent> {
    match value.get("type")?.as_str()? {
        "agent_start" => Some(AgentRuntimeEvent::Thinking { generation }),
        "agent_settled" => {
            let (total_tokens, total_cost_usd) = rpc_usage(value);
            Some(AgentRuntimeEvent::Settled {
                generation,
                total_tokens,
                total_cost_usd,
                has_visible_text: None,
            })
        }
        "agent_end" if value.get("willRetry").and_then(Value::as_bool) != Some(true) => {
            let (total_tokens, total_cost_usd) = agent_end_usage(value);
            Some(AgentRuntimeEvent::Settled {
                generation,
                total_tokens,
                total_cost_usd,
                has_visible_text: Some(agent_end_has_visible_text(value)),
            })
        }
        "message_update"
            if value
                .pointer("/assistantMessageEvent/type")
                .and_then(Value::as_str)
                == Some("thinking_start") =>
        {
            Some(AgentRuntimeEvent::ReasoningStarted { generation })
        }
        "message_update"
            if value
                .pointer("/assistantMessageEvent/type")
                .and_then(Value::as_str)
                == Some("thinking_delta") =>
        {
            Some(AgentRuntimeEvent::ReasoningDelta {
                generation,
                delta: value
                    .pointer("/assistantMessageEvent/delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        }
        "message_update"
            if value
                .pointer("/assistantMessageEvent/type")
                .and_then(Value::as_str)
                == Some("thinking_end") =>
        {
            Some(AgentRuntimeEvent::ReasoningFinished { generation })
        }
        "message_update"
            if value
                .pointer("/assistantMessageEvent/type")
                .and_then(Value::as_str)
                == Some("text_delta") =>
        {
            let (total_tokens, total_cost_usd) = rpc_usage(value);
            Some(AgentRuntimeEvent::TextDelta {
                generation,
                delta: value
                    .pointer("/assistantMessageEvent/delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                total_tokens,
                total_cost_usd,
            })
        }
        "tool_execution_start" => {
            let name = value
                .get("toolName")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            Some(AgentRuntimeEvent::ToolStarted {
                generation,
                call_id: value
                    .get("toolCallId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                detail: tool_call_detail(&name, value.get("args")),
                name,
            })
        }
        "tool_execution_end" => Some(AgentRuntimeEvent::ToolFinished {
            generation,
            call_id: value
                .get("toolCallId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            is_error: value
                .get("isError")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        "extension_ui_request" => Some(AgentRuntimeEvent::ExtensionUiRequest {
            generation,
            request_id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            method: value
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            title: value
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_string),
            payload: value
                .get("placeholder")
                .or_else(|| value.get("prefill"))
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        "response"
            if value.get("success").and_then(Value::as_bool) == Some(true)
                && value.get("command").and_then(Value::as_str) == Some("get_state") =>
        {
            Some(AgentRuntimeEvent::ModelContext {
                generation,
                model: value
                    .pointer("/data/model/id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                context_window: value
                    .pointer("/data/model/contextWindow")
                    .and_then(Value::as_u64),
            })
        }
        "response"
            if value.get("success").and_then(Value::as_bool) == Some(true)
                && value.get("command").and_then(Value::as_str) == Some("get_session_stats") =>
        {
            Some(AgentRuntimeEvent::ContextUsage {
                generation,
                context_tokens: value
                    .pointer("/data/contextUsage/tokens")
                    .and_then(Value::as_u64),
                context_window: value
                    .pointer("/data/contextUsage/contextWindow")
                    .and_then(Value::as_u64),
            })
        }
        "response"
            if value.get("success").and_then(Value::as_bool) == Some(false)
                && matches!(
                    value.get("command").and_then(Value::as_str),
                    Some("get_state" | "get_session_stats")
                ) =>
        {
            None
        }
        "response" if value.get("success").and_then(Value::as_bool) == Some(false) => {
            Some(AgentRuntimeEvent::Error {
                generation,
                message: value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("Pi rejected the request")
                    .to_string(),
            })
        }
        "extension_error" => Some(AgentRuntimeEvent::Error {
            generation,
            message: value
                .get("error")
                .and_then(Value::as_str)
                .or_else(|| value.get("message").and_then(Value::as_str))
                .unwrap_or("The Kerosene Pi extension failed")
                .to_string(),
        }),
        "message_end"
            if value.pointer("/message/stopReason").and_then(Value::as_str) == Some("error") =>
        {
            Some(AgentRuntimeEvent::Error {
                generation,
                message: value
                    .pointer("/message/errorMessage")
                    .and_then(Value::as_str)
                    .unwrap_or("The model request failed")
                    .to_string(),
            })
        }
        _ => None,
    }
}

fn rpc_usage(value: &Value) -> (Option<u64>, Option<f64>) {
    let usage = value
        .get("usage")
        .or_else(|| value.pointer("/message/usage"));
    let total_tokens = usage
        .and_then(|usage| usage.get("totalTokens"))
        .and_then(Value::as_u64);
    let total_cost_usd = usage
        .and_then(|usage| usage.pointer("/cost/total"))
        .and_then(Value::as_f64);
    (total_tokens, total_cost_usd)
}

fn agent_end_usage(value: &Value) -> (Option<u64>, Option<f64>) {
    let Some(messages) = value.get("messages").and_then(Value::as_array) else {
        return rpc_usage(value);
    };

    let mut total_tokens = None::<u64>;
    let mut total_cost_usd = None::<f64>;
    for message in messages {
        let (message_tokens, message_cost_usd) = rpc_usage(message);
        if let Some(message_tokens) = message_tokens {
            total_tokens = Some(
                total_tokens
                    .unwrap_or_default()
                    .saturating_add(message_tokens),
            );
        }
        if let Some(message_cost_usd) = message_cost_usd {
            total_cost_usd = Some(total_cost_usd.unwrap_or_default() + message_cost_usd);
        }
    }
    (total_tokens, total_cost_usd)
}

fn agent_end_has_visible_text(value: &Value) -> bool {
    value
        .get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .rev()
        .find(|message| message.get("role").and_then(Value::as_str) == Some("assistant"))
        .is_some_and(|message| {
            if let Some(text) = message.get("content").and_then(Value::as_str) {
                return !text.trim().is_empty();
            }
            message
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|part| {
                    part.get("type").and_then(Value::as_str) == Some("text")
                        && part
                            .get("text")
                            .and_then(Value::as_str)
                            .is_some_and(|text| !text.trim().is_empty())
                })
        })
}

#[cfg(test)]
mod tests;
