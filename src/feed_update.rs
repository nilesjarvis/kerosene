use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod connection;
mod liquidations;
mod tracked_trades;

impl TradingTerminal {
    pub(crate) fn update_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::HydromancerKeyInputChanged(_)
            | Message::SaveHydromancerKey
            | Message::ReconnectLiquidations
            | Message::ReconnectTrackedTrades) => return self.update_feed_connection(message),
            message @ (Message::WsHydromancerLiquidation(_) | Message::ClearLiquidations) => {
                return self.update_liquidation_feed(message);
            }
            message @ (Message::WsHydromancerTrackedTrades(_) | Message::ClearTrackedTrades) => {
                return self.update_tracked_trade_feed(message);
            }
            _ => {}
        }

        Task::none()
    }
}
