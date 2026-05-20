use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_pane_menu(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchBottomTab(tab) => {
                for (_pane, kind) in self.panes.iter_mut() {
                    if let PaneKind::BottomTabs { active_tab } = kind {
                        *active_tab = tab;
                    }
                }
            }
            Message::CloseAllMenus => {
                self.close_chart_header_menus();
            }
            Message::ToggleAddWidgetMenu => {
                let opening = !self.add_widget_menu_open;
                if opening {
                    self.close_chart_header_menus();
                }
                self.add_widget_menu_open = opening;
            }
            Message::ToggleLayoutMenu => {
                let opening = !self.layout_menu_open;
                if opening {
                    self.close_chart_header_menus();
                }
                self.layout_menu_open = opening;
            }
            Message::ToggleTickerTape => {
                self.add_widget_menu_open = false;
                self.ticker_tape_enabled = !self.ticker_tape_enabled;
                self.ticker_tape_scroll_px = 0.0;
                self.persist_config();
                return Task::batch([
                    self.request_ticker_tape_context_refresh(true),
                    self.sync_main_window_min_size(),
                ]);
            }
            Message::SetAddWidgetPlacement(placement) => {
                self.add_widget_placement = placement;
            }
            _ => {}
        }

        Task::none()
    }
}
