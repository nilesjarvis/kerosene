use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::Element;
use iced::widget::pane_grid;

// ---------------------------------------------------------------------------
// Pane Routing
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_pane_content(
        &self,
        pane: pane_grid::Pane,
        kind: &PaneKind,
        chart_count: usize,
    ) -> Element<'_, Message> {
        match kind {
            PaneKind::AccountSummary => self.view_account_summary(),
            PaneKind::Chart(id) => self.view_chart(*id, chart_count),
            PaneKind::OrderBook(id) => self.view_order_book(*id),
            PaneKind::LiveWatchlist(id) => self.view_live_watchlist(*id),
            PaneKind::PositioningInfo(id) => self.view_positioning_info(*id),
            PaneKind::Watchlist => self.view_watchlist(),
            PaneKind::Portfolio => self.view_portfolio(),
            PaneKind::Income => self.view_income(),
            PaneKind::BottomTabs { active_tab } => self.view_bottom_tabs(*active_tab),
            PaneKind::OrderEntry => self.view_order_entry(),
            PaneKind::AdvancedOrders => self.view_advanced_orders(),
            PaneKind::SpaghettiChart(id) => self.view_spaghetti_chart(*id, pane),
            PaneKind::Settings => self.view_settings_deprecated(),
            PaneKind::Calendar => self.view_calendar(),
            PaneKind::Liquidations => self.view_liquidations(),
            PaneKind::TrackedTrades => self.view_tracked_trades(),
            PaneKind::Outcomes => self.view_outcomes(),
        }
    }
}
