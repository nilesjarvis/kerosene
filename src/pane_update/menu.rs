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
                for inst in self.charts.values_mut() {
                    inst.macro_menu_open = false;
                }
                self.add_widget_menu_open = false;
                self.account_picker_open = false;
            }
            Message::ToggleAddWidgetMenu => {
                self.add_widget_menu_open = !self.add_widget_menu_open;
                if self.add_widget_menu_open {
                    self.account_picker_open = false;
                }
            }
            Message::SetAddWidgetPlacement(placement) => {
                self.add_widget_placement = placement;
            }
            _ => {}
        }

        Task::none()
    }
}
