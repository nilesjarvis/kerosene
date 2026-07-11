mod routing;

#[cfg(test)]
mod tests;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

use self::routing::{UpdateRoute, message_route};

/// Fresh UI/command intents that can dispatch a new signed exchange mutation or
/// begin destructive persistence work.
///
/// Result, status, and explicit cleanup messages are deliberately excluded:
/// final exit must keep reconciling work that was already sent and must still
/// be able to reduce known resting exposure.
fn is_fresh_mutation_intent_fenced_during_exit(message: &Message) -> bool {
    matches!(
        message,
        Message::SubmitOrderLeverage(_)
            | Message::ExecutePreset(_, _, _)
            | Message::PlaceOrder { .. }
            | Message::ClosePosition { .. }
            | Message::NukePositions
            | Message::StartChase { .. }
            | Message::StartTwap { .. }
            | Message::SubmitQuickOrder { .. }
            | Message::SubmitHudOrder(_)
            | Message::MoveOrder { .. }
            | Message::ChaseRestingOrder { .. }
            | Message::AlfredSubmit
            | Message::AlfredCommandSelected(_)
            | Message::WalletClusterSubmitOrder { .. }
            | Message::WalletClusterClosePosition { .. }
            | Message::ClearConfigs
    )
}

impl TradingTerminal {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        if self.config_save_exit_requested && is_fresh_mutation_intent_fenced_during_exit(&message)
        {
            return Task::none();
        }

        let _theme = self.theme();
        match message_route(&message) {
            UpdateRoute::Alfred => self.update_alfred(message),
            UpdateRoute::Layout => self.update_layout(message),
            UpdateRoute::PaneInteractions => self.update_pane_interactions(message),
            UpdateRoute::Panes => self.update_panes(message),
            UpdateRoute::Chrome => self.update_chrome(message),
            UpdateRoute::Order => self.update_order(message),
            UpdateRoute::Market => self.update_market(message),
            UpdateRoute::Preferences => self.update_preferences(message),
            UpdateRoute::Screener => self.update_screener(message),
            UpdateRoute::Settings => self.update_settings(message),
            UpdateRoute::Calendar => self.update_calendar(message),
            UpdateRoute::Window => self.update_window(message),
            UpdateRoute::Journal => self.update_journal(message),
            UpdateRoute::Spaghetti => self.update_spaghetti(message),
            UpdateRoute::WalletCluster => self.update_wallet_cluster(message),
            UpdateRoute::WalletTracker => self.update_wallet_tracker(message),
            UpdateRoute::PortfolioIncome => self.update_portfolio_income(message),
            UpdateRoute::Annotations => self.update_annotations(message),
            UpdateRoute::Chart => self.update_chart(message),
            UpdateRoute::ChartScreenshot => self.update_chart_screenshot(message),
            UpdateRoute::Account => self.update_account(message),
            UpdateRoute::Feed => self.update_feed(message),
            UpdateRoute::Hyperdash => self.update_hyperdash(message),
            UpdateRoute::OpenRouter => self.update_openrouter(message),
        }
    }
}
