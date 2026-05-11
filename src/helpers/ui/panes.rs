use crate::pane_state::PaneKind;

// ---------------------------------------------------------------------------
// Pane Titles
// ---------------------------------------------------------------------------

pub fn pane_title(kind: &PaneKind) -> String {
    match kind {
        PaneKind::AccountSummary => "Account Summary".to_string(),
        PaneKind::Chart(id) => {
            // Title will be set dynamically in view, but provide a default.
            format!("Chart #{id}")
        }
        PaneKind::OrderBook(_) => "Order Book".to_string(),
        PaneKind::Watchlist => "Symbol Search".to_string(),
        PaneKind::Portfolio => "Portfolio".to_string(),
        PaneKind::Income => "Income".to_string(),
        PaneKind::BottomTabs { .. } => "Positions / History".to_string(),
        PaneKind::OrderEntry => "Order Entry".to_string(),
        PaneKind::AdvancedOrders => "Advanced Orders".to_string(),
        PaneKind::SpaghettiChart(_) => "Comparison Chart".to_string(),
        PaneKind::Settings => "Theme & Settings".to_string(),
        PaneKind::Calendar => "Economic Calendar".to_string(),
        PaneKind::LiveWatchlist(_) => "Live Watchlist".to_string(),
        PaneKind::Liquidations => "Liquidations".to_string(),
        PaneKind::TrackedTrades => "Tracked Trades".to_string(),
        PaneKind::Outcomes => "Outcomes".to_string(),
    }
}
