use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::ws_open_interest_caps;
use iced::Subscription;

impl TradingTerminal {
    pub(super) fn push_chart_open_interest_cap_subscription(
        &self,
        subs: &mut Vec<Subscription<Message>>,
    ) {
        if !self.chart_open_interest_cap_dexes().is_empty() {
            // One native stream covers every DEX and survives account/provider changes.
            subs.push(
                Subscription::run(ws_open_interest_caps).map(Message::ChartOpenInterestCapStream),
            );
        }
    }
}
