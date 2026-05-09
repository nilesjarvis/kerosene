mod tool_results;

use self::tool_results::execute_tool_call;
use super::ollama::{OllamaChatMessage, chat_once};
use super::preview_tool_call;
use super::{AssistantPlannedTurn, AssistantTurnResult};

pub async fn execute_planned_turn(
    planned: AssistantPlannedTurn,
) -> Result<AssistantTurnResult, String> {
    let mut trace_lines: Vec<String> = Vec::new();
    trace_lines.push(format!(
        "Tool call: {}",
        preview_tool_call(&planned.tool_call)
    ));

    let tool_result = execute_tool_call(&planned.tool_call).await?;

    let code_exec_note = if planned.allow_code_execution {
        "Code execution toggle is ON, but dynamic code execution is intentionally disabled in this build."
    } else {
        "Code execution toggle is OFF."
    };

    let responder_prompt = format!(
        "Use the plan and tool output to answer clearly with assumptions and caveats.\n\nPlan:\n{}\n\nTool output:\n{}\n\n{}",
        planned.plan_text, tool_result, code_exec_note
    );

    let answer_text = if planned.model.trim().is_empty() {
        tool_result
    } else {
        chat_once(
            &planned.ollama_url,
            &planned.model,
            vec![
                OllamaChatMessage {
                    role: "system".to_string(),
                    content: "You are a quantitative trading assistant. Never invent data; rely on tool output.".to_string(),
                },
                OllamaChatMessage {
                    role: "user".to_string(),
                    content: responder_prompt,
                },
            ],
        )
        .await?
    };

    Ok(AssistantTurnResult {
        trace_lines,
        answer_text,
    })
}
