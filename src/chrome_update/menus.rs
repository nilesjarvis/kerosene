use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};

// ---------------------------------------------------------------------------
// Shared Chrome Menus
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn dismiss_chart_toolbar_menu(
        &mut self,
        chart_id: ChartId,
        surface: ChartSurfaceId,
    ) -> bool {
        self.charts.get_mut(&chart_id).is_some_and(|instance| {
            if instance.chart.surface_id() == surface
                && std::mem::take(&mut instance.macro_menu_open)
            {
                return true;
            }
            instance
                .drawing_menu
                .take_if(|(owner, _)| *owner == surface)
                .is_some()
        })
    }

    pub(crate) fn close_chart_header_menus(&mut self) {
        for inst in self.charts.values_mut() {
            inst.macro_menu_open = false;
            inst.drawing_menu = None;
        }
        for inst in self.spaghetti_charts.values_mut() {
            inst.style_menu_open = false;
        }
        self.add_widget_menu_open = false;
        self.layout_menu_open = false;
        self.layout_rename_index = None;
        self.layout_rename_input.clear();
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.chart_screenshot_menu_open = None;
        self.tracked_trade_settings_menu_open = false;
        self.liquidation_settings_menu_open = false;
        self.live_watchlist_settings_menu_open = None;
    }
}
