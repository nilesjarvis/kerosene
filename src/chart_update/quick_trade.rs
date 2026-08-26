use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, QuickTradeActionDraft, QuickTradeEditorState};
use crate::config::{
    MAX_QUICK_TRADE_ACTIONS, QuickTradeActionConfig, QuickTradeDenomination, QuickTradeSide,
};
use crate::message::Message;

use iced::{Size, Task, window};

// ---------------------------------------------------------------------------
// Quick Trade Action Editor
// ---------------------------------------------------------------------------

const QUICK_TRADE_EDITOR_SIZE: Size = Size {
    width: 620.0,
    height: 460.0,
};
const QUICK_TRADE_EDITOR_MIN_SIZE: Size = Size {
    width: 520.0,
    height: 360.0,
};

impl TradingTerminal {
    pub(super) fn update_chart_quick_trade(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenQuickTradeEditor(chart_id) => {
                return self.open_quick_trade_editor(chart_id);
            }
            Message::QuickTradeActionAdded => {
                let Some(editor) = self.quick_trade_editor.as_mut() else {
                    return Task::none();
                };
                if editor.actions.len() >= MAX_QUICK_TRADE_ACTIONS {
                    editor.error = Some(format!(
                        "Quick Trade supports up to {MAX_QUICK_TRADE_ACTIONS} actions per chart"
                    ));
                } else {
                    editor.actions.push(QuickTradeActionDraft::empty());
                    editor.error = None;
                }
            }
            Message::QuickTradeActionSideToggled(index) => {
                if let Some(editor) = self.quick_trade_editor.as_mut()
                    && let Some(action) = editor.actions.get_mut(index)
                {
                    action.side = match action.side {
                        QuickTradeSide::Buy => QuickTradeSide::Sell,
                        QuickTradeSide::Sell => QuickTradeSide::Buy,
                    };
                    editor.error = None;
                }
            }
            Message::QuickTradeActionDenominationToggled(index) => {
                if let Some(editor) = self.quick_trade_editor.as_mut()
                    && let Some(action) = editor.actions.get_mut(index)
                {
                    action.denomination = match action.denomination {
                        QuickTradeDenomination::Usd => QuickTradeDenomination::Coin,
                        QuickTradeDenomination::Coin => QuickTradeDenomination::Usd,
                    };
                    editor.error = None;
                }
            }
            Message::QuickTradeActionQuantityChanged(index, value) => {
                if let Some(editor) = self.quick_trade_editor.as_mut()
                    && let Some(action) = editor.actions.get_mut(index)
                {
                    action.quantity = value.into_string();
                    editor.error = None;
                }
            }
            Message::QuickTradeActionRemoved(index) => {
                if let Some(editor) = self.quick_trade_editor.as_mut()
                    && index < editor.actions.len()
                {
                    editor.actions.remove(index);
                    editor.error = None;
                }
            }
            Message::SaveQuickTradeActions => return self.save_quick_trade_actions(),
            Message::CloseQuickTradeEditor => return self.close_quick_trade_editor(),
            _ => {}
        }

        Task::none()
    }

    fn open_quick_trade_editor(&mut self, chart_id: ChartId) -> Task<Message> {
        let Some(instance) = self.charts.get(&chart_id) else {
            self.push_toast("Quick Trade chart is no longer available".to_string(), true);
            return Task::none();
        };

        if let Some((window_id, editor_chart_id)) = self
            .quick_trade_editor
            .as_ref()
            .map(|editor| (editor.window_id, editor.chart_id))
        {
            if editor_chart_id != chart_id {
                self.push_toast(
                    "Finish or cancel the open Quick Trade editor first".to_string(),
                    true,
                );
            }
            return window::gain_focus(window_id);
        }

        let actions = instance.quick_trade_actions.clone();
        let settings = window::Settings {
            size: QUICK_TRADE_EDITOR_SIZE,
            min_size: Some(QUICK_TRADE_EDITOR_MIN_SIZE),
            ..crate::window_chrome::settings(
                self.custom_window_chrome_active,
                self.window_background_blur_enabled,
            )
        };
        let (window_id, task) = window::open(settings);
        self.quick_trade_editor = Some(QuickTradeEditorState::new(window_id, chart_id, &actions));
        task.map(Message::WindowOpened)
    }

    fn save_quick_trade_actions(&mut self) -> Task<Message> {
        let Some(editor) = self.quick_trade_editor.as_ref() else {
            return Task::none();
        };

        let mut actions = Vec::with_capacity(editor.actions.len());
        for (index, draft) in editor.actions.iter().enumerate() {
            let Some(quantity) = crate::helpers::parse_positive_number(&draft.quantity) else {
                if let Some(editor) = self.quick_trade_editor.as_mut() {
                    editor.error = Some(format!(
                        "Action {} needs a positive, finite quantity",
                        index + 1
                    ));
                }
                return Task::none();
            };
            actions.push(QuickTradeActionConfig {
                side: draft.side,
                quantity,
                denomination: draft.denomination,
            });
        }

        let chart_id = editor.chart_id;
        let Some(instance) = self.charts.get_mut(&chart_id) else {
            if let Some(editor) = self.quick_trade_editor.as_mut() {
                editor.error = Some("The chart was removed before these actions were saved".into());
            }
            return Task::none();
        };
        instance.quick_trade_actions = actions;
        self.persist_config();
        self.close_quick_trade_editor()
    }

    fn close_quick_trade_editor(&mut self) -> Task<Message> {
        let Some(editor) = self.quick_trade_editor.take() else {
            return Task::none();
        };
        window::close(editor.window_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    fn terminal_with_editor() -> TradingTerminal {
        let (mut terminal, _) = TradingTerminal::boot();
        let chart_id = 7;
        terminal.charts.insert(
            chart_id,
            ChartInstance::new(chart_id, "BTC".to_string(), Timeframe::H1),
        );
        let window_id = window::Id::unique();
        terminal.quick_trade_editor = Some(QuickTradeEditorState::new(window_id, chart_id, &[]));
        terminal
    }

    #[test]
    fn editor_saves_valid_actions_to_the_target_chart() {
        let mut terminal = terminal_with_editor();
        let _ = terminal.update_chart_quick_trade(Message::QuickTradeActionAdded);
        let _ = terminal
            .update_chart_quick_trade(Message::QuickTradeActionQuantityChanged(0, "10000".into()));

        let _task = terminal.update_chart_quick_trade(Message::SaveQuickTradeActions);

        let actions = &terminal.charts[&7].quick_trade_actions;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].quantity, 10_000.0);
        assert_eq!(actions[0].side, QuickTradeSide::Buy);
        assert_eq!(actions[0].denomination, QuickTradeDenomination::Usd);
        assert!(terminal.quick_trade_editor.is_none());
    }

    #[test]
    fn editor_rejects_invalid_quantity_without_overwriting_actions() {
        let mut terminal = terminal_with_editor();
        terminal
            .charts
            .get_mut(&7)
            .expect("chart")
            .quick_trade_actions = vec![QuickTradeActionConfig {
            side: QuickTradeSide::Sell,
            quantity: 1.0,
            denomination: QuickTradeDenomination::Coin,
        }];
        let _ = terminal.update_chart_quick_trade(Message::QuickTradeActionAdded);

        let _task = terminal.update_chart_quick_trade(Message::SaveQuickTradeActions);

        assert_eq!(terminal.charts[&7].quick_trade_actions.len(), 1);
        assert!(
            terminal
                .quick_trade_editor
                .as_ref()
                .and_then(|editor| editor.error.as_ref())
                .is_some_and(|error| error.contains("positive"))
        );
    }
}
