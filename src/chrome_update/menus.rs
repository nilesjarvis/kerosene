use crate::app_state::TradingTerminal;

// ---------------------------------------------------------------------------
// Shared Chrome Menus
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn close_chart_header_menus(&mut self) {
        for inst in self.charts.values_mut() {
            inst.macro_menu_open = false;
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
