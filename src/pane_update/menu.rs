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
            Message::SetAddWidgetPlacement(placement) => {
                self.add_widget_placement = placement;
            }
            _ => {}
        }

        Task::none()
    }
}
