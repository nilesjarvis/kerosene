mod routing;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

use self::routing::{UpdateRoute, message_route};

impl TradingTerminal {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        let _theme = self.theme();
        match message_route(&message) {
            UpdateRoute::Layout => self.update_layout(message),
            UpdateRoute::PaneInteractions => self.update_pane_interactions(message),
            UpdateRoute::Panes => self.update_panes(message),
            UpdateRoute::Chrome => self.update_chrome(message),
            UpdateRoute::Order => self.update_order(message),
            UpdateRoute::Market => self.update_market(message),
            UpdateRoute::Preferences => self.update_preferences(message),
            UpdateRoute::Settings => self.update_settings(message),
            UpdateRoute::Calendar => self.update_calendar(message),
            UpdateRoute::Window => self.update_window(message),
            UpdateRoute::Journal => self.update_journal(message),
            UpdateRoute::Spaghetti => self.update_spaghetti(message),
            UpdateRoute::WalletTracker => self.update_wallet_tracker(message),
            UpdateRoute::PortfolioIncome => self.update_portfolio_income(message),
            UpdateRoute::Annotations => self.update_annotations(message),
            UpdateRoute::Chart => self.update_chart(message),
            UpdateRoute::ChartScreenshot => self.update_chart_screenshot(message),
            UpdateRoute::Account => self.update_account(message),
            UpdateRoute::Feed => self.update_feed(message),
            UpdateRoute::Hyperdash => self.update_hyperdash(message),
        }
    }
}
