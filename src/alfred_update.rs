use crate::alfred_state::{
    AlfredCommand, AlfredCommandId, AlfredSelectionStep, alfred_query_is_nuke,
};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::OrderKind;
use iced::Task;

// ---------------------------------------------------------------------------
// Alfred update
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn update_alfred(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleAlfred => {
                if self.alfred.open {
                    self.alfred.close();
                    Task::none()
                } else {
                    self.open_alfred()
                }
            }
            Message::CloseAlfred => {
                self.alfred.close();
                Task::none()
            }
            Message::AlfredQueryChanged(query) => {
                self.alfred.query = query;
                self.alfred.selected_index = 0;
                Task::none()
            }
            Message::AlfredSelectionMoved(step) => {
                self.move_alfred_selection(step);
                Task::none()
            }
            Message::AlfredSubmit => self.submit_selected_alfred_command(),
            Message::AlfredCommandSelected(id) => self.submit_alfred_command(id),
            _ => Task::none(),
        }
    }

    pub(crate) fn handle_alfred_keyboard(
        &mut self,
        key: iced::keyboard::Key<&str>,
        modifiers: iced::keyboard::Modifiers,
        status: iced::event::Status,
    ) -> Task<Message> {
        if modifiers.control() || modifiers.alt() || modifiers.logo() {
            return Task::none();
        }

        match key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                self.update(Message::CloseAlfred)
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Next))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if !modifiers.shift() => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Next))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Previous))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) if modifiers.shift() => {
                self.update(Message::AlfredSelectionMoved(AlfredSelectionStep::Previous))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
                if status == iced::event::Status::Ignored =>
            {
                self.update(Message::AlfredSubmit)
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn alfred_input_id() -> iced::widget::Id {
        iced::widget::Id::from("alfred_input")
    }

    fn open_alfred(&mut self) -> Task<Message> {
        self.close_chart_header_menus();
        self.add_widget_menu_open = false;
        self.layout_menu_open = false;
        self.layout_rename_index = None;
        self.layout_rename_input.clear();
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.alfred.open = true;
        self.alfred.query.clear();
        self.alfred.selected_index = 0;

        iced::widget::operation::focus(Self::alfred_input_id())
    }

    fn move_alfred_selection(&mut self, step: AlfredSelectionStep) {
        let result_count = self.alfred_filtered_commands().len();
        if result_count == 0 {
            self.alfred.selected_index = 0;
            return;
        }

        let current = self.alfred.selected_index.min(result_count - 1);
        self.alfred.selected_index = match step {
            AlfredSelectionStep::Previous => current.saturating_sub(1),
            AlfredSelectionStep::Next => current.saturating_add(1).min(result_count - 1),
        };
    }

    fn submit_selected_alfred_command(&mut self) -> Task<Message> {
        let commands = self.alfred_filtered_commands();
        let Some(command) = selected_command(&commands, self.alfred.selected_index) else {
            self.push_toast("No Alfred matches".to_string(), true);
            return Task::none();
        };

        self.submit_alfred_command(command.id)
    }

    fn submit_alfred_command(&mut self, id: AlfredCommandId) -> Task<Message> {
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

        self.order_kind = draft.order_kind;
        self.order_quantity_is_usd = draft.quantity_is_usd && !self.is_outcome_coin(&symbol_key);
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

        self.alfred.close();
        self.close_menu_coin = None;
        self.nuke_confirmation = None;
        self.execute_nuke_positions()
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
