use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti::ComparisonColorMode;
use crate::spaghetti_state::SpaghettiChartId;

use iced::Task;

impl TradingTerminal {
    pub(super) fn toggle_spaghetti_style_menu(&mut self, id: SpaghettiChartId) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.style_menu_open = !inst.style_menu_open;
        }
        Task::none()
    }

    pub(super) fn toggle_spaghetti_labels(&mut self, id: SpaghettiChartId) -> Task<Message> {
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            if inst.canvas.color_mode == ComparisonColorMode::Single {
                inst.canvas.show_labels = true;
            } else {
                inst.canvas.show_labels = !inst.canvas.show_labels;
            }
            inst.canvas.cache.clear();
            self.persist_config();
        }
        Task::none()
    }

    pub(super) fn set_spaghetti_color_mode(
        &mut self,
        id: SpaghettiChartId,
        mode: ComparisonColorMode,
    ) -> Task<Message> {
        let theme = self.theme();
        if let Some(inst) = self.spaghetti_charts.get_mut(&id) {
            inst.canvas.color_mode = mode;
            if mode == ComparisonColorMode::Single {
                inst.canvas.show_labels = true;
            }
            inst.canvas.apply_style_colors(&theme);
            self.persist_config();
        }
        Task::none()
    }
}
