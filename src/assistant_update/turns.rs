use crate::app_state::TradingTerminal;
use crate::assistant::{
    self, AssistantChatMessage, AssistantRole, AssistantRuntimeContext, AssistantTurnInput,
};
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(super) fn send_assistant_prompt(&mut self) -> Task<Message> {
        let prompt = self.assistant.input.trim().to_string();
        if prompt.is_empty() {
            return Task::none();
        }

        let model = self.assistant.selected_model.clone().unwrap_or_default();
        if model.is_empty() && !assistant::is_simple_price_query(&prompt) {
            self.assistant.last_error = Some("Select a local Ollama model first".to_string());
            return Task::none();
        }

        self.assistant.history.push(AssistantChatMessage {
            role: AssistantRole::User,
            content: prompt.clone(),
        });
        self.assistant.input.clear();
        self.assistant.loading = true;
        self.assistant.status_line = Some("Planning request...".to_string());
        self.assistant.last_error = None;

        let latest_price = self
            .active_mark_price_for_symbol(&self.active_symbol)
            .or_else(|| self.resolve_mid_for_symbol(&self.active_symbol));
        let account_summary = self.account_data.as_ref().map(|data| {
            let positions = data
                .clearinghouse
                .asset_positions
                .iter()
                .filter(|position| !self.is_ticker_muted(&position.position.coin))
                .count();
            let orders = data
                .open_orders
                .iter()
                .filter(|order| !self.is_ticker_muted(&order.coin))
                .count();
            let fills = data
                .fills
                .iter()
                .filter(|fill| !self.is_ticker_muted(&fill.coin))
                .count();
            format!("positions={positions}; open_orders={orders}; recent_fills={fills}")
        });
        let context = AssistantRuntimeContext {
            active_symbol: self.active_symbol.clone(),
            active_timeframe: self
                .primary_chart_id
                .and_then(|id| self.charts.get(&id))
                .map(|inst| inst.interval.api_str().to_string())
                .unwrap_or_else(|| "1h".to_string()),
            latest_price,
            account_summary,
            connected_address: self.connected_address.clone(),
            hyperdash_api_key: if self.hyperdash_api_key.trim().is_empty() {
                None
            } else {
                Some(self.hyperdash_api_key.to_string())
            },
        };

        let input = AssistantTurnInput {
            ollama_url: self.assistant.ollama_url.clone(),
            model,
            user_prompt: prompt,
            context,
            use_account_context: self.assistant.use_account_context,
            allow_code_execution: self.assistant.allow_code_execution,
        };
        Task::perform(
            async move { assistant::plan_turn(input).await },
            Message::AssistantPlanLoaded,
        )
    }

    pub(super) fn apply_assistant_plan_loaded(
        &mut self,
        result: Result<assistant::AssistantPlannedTurn, String>,
    ) -> Task<Message> {
        match result {
            Ok(planned) => {
                self.assistant.history.push(AssistantChatMessage {
                    role: AssistantRole::System,
                    content: format!("Plan\n{}", planned.plan_text),
                });
                let api_preview = assistant::preview_tool_call(&planned.tool_call);
                self.assistant.history.push(AssistantChatMessage {
                    role: AssistantRole::System,
                    content: api_preview,
                });
                self.assistant.status_line = Some("Running tools...".to_string());
                Task::perform(
                    async move { assistant::execute_planned_turn(planned).await },
                    Message::AssistantExecuteLoaded,
                )
            }
            Err(error) => {
                self.assistant.loading = false;
                self.assistant.status_line = None;
                self.assistant.last_error = Some(error.clone());
                self.assistant.history.push(AssistantChatMessage {
                    role: AssistantRole::Assistant,
                    content: format!("Planning failed: {error}"),
                });
                Task::none()
            }
        }
    }

    pub(super) fn apply_assistant_execute_loaded(
        &mut self,
        result: Result<assistant::AssistantTurnResult, String>,
    ) -> Task<Message> {
        self.assistant.loading = false;
        self.assistant.status_line = None;
        match result {
            Ok(turn) => {
                for line in turn.trace_lines {
                    self.assistant.history.push(AssistantChatMessage {
                        role: AssistantRole::System,
                        content: line,
                    });
                }
                self.assistant.history.push(AssistantChatMessage {
                    role: AssistantRole::Assistant,
                    content: turn.answer_text,
                });
            }
            Err(error) => {
                self.assistant.last_error = Some(error.clone());
                self.assistant.history.push(AssistantChatMessage {
                    role: AssistantRole::Assistant,
                    content: format!("Request failed: {error}"),
                });
            }
        }
        Task::none()
    }
}
