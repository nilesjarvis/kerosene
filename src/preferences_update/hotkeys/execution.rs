use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::config;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

impl TradingTerminal {
    pub(super) fn execute_configured_hotkey(&mut self, message: Message) -> Task<Message> {
        let Message::ExecuteHotkey(action) = message else {
            return Task::none();
        };

        match action {
            config::HotkeyAction::AddCandlestickChart => self.add_chart_from_hotkey(),
            config::HotkeyAction::OpenTradingJournal => self.update(Message::AddTradingJournal),
            config::HotkeyAction::OpenWalletTracker => {
                self.update(Message::OpenWalletTrackerWindow)
            }
            config::HotkeyAction::OpenQuickSymbolSearch => self.open_quick_symbol_search(),
            config::HotkeyAction::OpenSettingsWindow => self.update(Message::OpenSettingsWindow),
            config::HotkeyAction::SwitchLayout { name } => {
                let layout = self
                    .saved_layouts
                    .iter()
                    .find(|layout| layout.name == name)
                    .cloned();
                if let Some(layout) = layout {
                    return self.update(Message::LoadLayout(layout));
                }
                self.push_toast("Layout hotkey target no longer exists".to_string(), true);
                Task::none()
            }
            config::HotkeyAction::SwitchAccount { secret_id } => {
                if let Some(index) = self.account_index_for_secret_id(&secret_id) {
                    return self.switch_account_task(index);
                }
                self.push_toast("Account hotkey target no longer exists".to_string(), true);
                Task::none()
            }
        }
    }

    fn add_chart_from_hotkey(&mut self) -> Task<Message> {
        let Some(pane) = self.add_target_pane() else {
            self.push_toast(
                "Could not add Candlestick Chart: no pane is available".to_string(),
                true,
            );
            return Task::none();
        };
        let id = self.alloc_chart_id();
        let mut instance = ChartInstance::new_empty(id);
        let (bull, bear) = self.active_chart_theme_colors();
        instance.chart.set_chart_colors(bull, bear);
        self.charts.insert(id, instance);
        if self
            .add_pane_to_target(
                self.add_widget_axis(),
                pane,
                PaneKind::Chart(id),
                "Candlestick Chart",
            )
            .is_some()
        {
            self.primary_chart_id = Some(id);
            return iced::widget::operation::focus(Self::chart_symbol_search_input_id(id));
        }
        self.charts.remove(&id);

        Task::none()
    }
}
