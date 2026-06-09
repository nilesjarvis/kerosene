use crate::alfred_state::{AlfredCommand, AlfredCommandId, alfred_query_is_nuke};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_update::nuke_confirmation_is_armed;
use crate::signing::OrderKind;
use iced::Task;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Alfred Command Submission
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn submit_selected_alfred_command(&mut self) -> Task<Message> {
        let commands = self.alfred_filtered_commands();
        let Some(command) = selected_command(&commands, self.alfred.selected_index) else {
            self.push_toast("No Alfred matches".to_string(), true);
            return Task::none();
        };

        self.submit_alfred_command(command.id)
    }

    pub(super) fn submit_alfred_command(&mut self, id: AlfredCommandId) -> Task<Message> {
        if id == AlfredCommandId::NaturalLanguageTrading {
            return self.submit_alfred_trade();
        }
        if id == AlfredCommandId::NukePositions {
            return self.submit_alfred_nuke();
        }
        if id == AlfredCommandId::ClosePosition {
            return self.submit_alfred_close_position();
        }

        let Some(command) = self.alfred_command_by_id(id) else {
            self.push_toast("Alfred command is no longer available".to_string(), true);
            return Task::none();
        };

        if !command.enabled {
            self.push_toast(
                command
                    .disabled_reason
                    .unwrap_or_else(|| "Alfred command is not available yet".to_string()),
                true,
            );
            return Task::none();
        }

        let Some(message) = command.message else {
            self.push_toast("Alfred command is not wired yet".to_string(), true);
            return Task::none();
        };

        self.alfred.close();
        self.update(message)
    }

    fn submit_alfred_trade(&mut self) -> Task<Message> {
        let query = self.alfred.query.clone();
        let Some(draft) = self.alfred_trade_draft(&query) else {
            self.push_toast("Type a trade like 'buy 1k HYPE'".to_string(), true);
            return Task::none();
        };
        if !draft.can_submit() {
            let message = draft
                .error
                .unwrap_or_else(|| "Complete the trade before submitting".to_string());
            self.push_toast(message, true);
            return Task::none();
        }

        let Some(symbol_key) = draft.symbol_key.clone() else {
            self.push_toast("Add a symbol".to_string(), true);
            return Task::none();
        };

        self.alfred.close();
        let switch_task = if self.active_symbol == symbol_key {
            Task::none()
        } else {
            self.switch_active_symbol_internal(symbol_key.clone())
        };
        if self.active_symbol != symbol_key {
            self.push_toast(format!("Cannot trade {symbol_key}"), true);
            return switch_task;
        }
        if draft.quantity_is_usd && self.is_outcome_coin(&symbol_key) {
            let message = "USD sizing is not supported for outcome markets; use contracts";
            self.order_status = Some((message.to_string(), true));
            self.push_toast(message.to_string(), true);
            return switch_task;
        }

        self.order_kind = draft.order_kind;
        self.order_quantity_is_usd = draft.quantity_is_usd;
        self.order_price = match draft.order_kind {
            OrderKind::Limit => draft.limit_price_input().unwrap_or_default(),
            OrderKind::Market => String::new(),
            OrderKind::LimitIoc | OrderKind::Chase | OrderKind::Twap => String::new(),
        };
        self.presets_menu_expanded = false;
        self.handle_order_quantity_changed(draft.quantity_input());
        self.persist_config();

        if let Some(side) = draft.side {
            return Task::batch([switch_task, self.execute_order(side.is_buy())]);
        }

        if draft.order_kind == OrderKind::Chase {
            self.order_status = Some((
                format!("Chase draft ready for {symbol_key}: choose CHASE BUY or CHASE SELL"),
                false,
            ));
            self.push_toast(format!("Chase draft ready for {symbol_key}"), false);
            return switch_task;
        }

        self.push_toast("Start with buy or sell".to_string(), true);
        switch_task
    }

    fn submit_alfred_nuke(&mut self) -> Task<Message> {
        let query = self.alfred.query.clone();
        let Some(command) = self.alfred_command_by_id(AlfredCommandId::NukePositions) else {
            self.push_toast(
                "Type 'nuke' or 'close all' to close open positions".to_string(),
                true,
            );
            return Task::none();
        };

        if !alfred_query_is_nuke(&query) || !command.enabled {
            self.push_toast(
                command
                    .disabled_reason
                    .unwrap_or_else(|| "NUKE is not available".to_string()),
                true,
            );
            return Task::none();
        }

        // Route through the same two-press arming flow as the NUKE button so
        // a single Enter in the palette can never flatten every position.
        let was_armed = nuke_confirmation_is_armed(self.nuke_confirmation, Instant::now());
        let task = self.handle_nuke_positions();
        if was_armed {
            // Second press: the nuke is executing; the palette's job is done.
            self.alfred.close();
        } else if let Some((status, is_error)) = self.order_status.clone() {
            // First press armed (or refused to arm); echo the plan where the
            // user is looking and keep the palette open for the confirm press.
            self.push_toast(status, is_error);
        }
        task
    }

    fn submit_alfred_close_position(&mut self) -> Task<Message> {
        let query = self.alfred.query.clone();
        let Some(draft) = self.alfred_close_position_draft(&query) else {
            self.push_toast("Type 'close HYPE' to close a position".to_string(), true);
            return Task::none();
        };
        if !draft.can_submit() {
            self.push_toast(
                draft
                    .error
                    .unwrap_or_else(|| "Complete the close command before submitting".to_string()),
                true,
            );
            return Task::none();
        }

        let Some(coin) = draft.coin else {
            self.push_toast("Add a ticker to close".to_string(), true);
            return Task::none();
        };

        self.alfred.close();
        self.close_menu_coin = None;
        self.execute_close_position(&coin, draft.fraction, true)
    }
}

fn selected_command(commands: &[AlfredCommand], selected_index: usize) -> Option<&AlfredCommand> {
    let index = selected_index.min(commands.len().checked_sub(1)?);
    commands.get(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

    fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage: 50,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    fn outcome_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            market_type: MarketType::Outcome,
            outcome: Some(OutcomeSymbolInfo {
                outcome_id: 66,
                question_id: Some(12),
                question_name: Some("Recurring".to_string()),
                question_description: None,
                question_class: Some("priceBucket".to_string()),
                question_underlying: Some("BTC".to_string()),
                question_expiry: Some("20260520-0600".to_string()),
                question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
                question_period: Some("1d".to_string()),
                question_named_outcomes: vec![67, 68, 69],
                question_settled_named_outcomes: Vec::new(),
                question_fallback_outcome: Some(66),
                bucket_index: Some(0),
                is_question_fallback: false,
                side_index: 0,
                side_name: "Yes".to_string(),
                outcome_name: "Recurring Named Outcome".to_string(),
                description: "index:0".to_string(),
                class: None,
                underlying: None,
                expiry: None,
                target_price: None,
                period: None,
                quote_symbol: "USDH".to_string(),
                quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
                encoding: 660,
            }),
            ..symbol(key, MarketType::Outcome)
        }
    }

    #[test]
    fn alfred_trade_rejects_usd_sizing_for_outcome_markets() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#660")];
        terminal.active_symbol = "#660".to_string();
        terminal.order_quantity = "old".to_string();
        terminal.order_quantity_is_usd = false;
        terminal.alfred.open = true;
        terminal.alfred.query = "buy $10 #660".to_string();

        let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

        assert_eq!(
            terminal.order_status,
            Some((
                "USD sizing is not supported for outcome markets; use contracts".to_string(),
                true
            ))
        );
        assert_eq!(terminal.order_quantity, "old");
        assert!(!terminal.order_quantity_is_usd);
    }
}
