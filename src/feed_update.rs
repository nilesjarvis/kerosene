use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

mod connection;
mod liquidations;
mod telegram;
mod tracked_trades;

impl TradingTerminal {
    pub(crate) fn update_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            message @ (Message::HydromancerKeyInputChanged(_)
            | Message::SaveHydromancerKey
            | Message::ReconnectLiquidations
            | Message::ReconnectTrackedTrades) => return self.update_feed_connection(message),
            message @ (Message::WsHydromancerLiquidation { .. }
            | Message::ClearLiquidations
            | Message::LiquidationFeedScrolled(_)) => {
                return self.update_liquidation_feed(message);
            }
            message
            @ (Message::WsHydromancerTrackedTrades { .. } | Message::ClearTrackedTrades) => {
                return self.update_tracked_trade_feed(message);
            }
            message @ (Message::RefreshTelegramFeed
            | Message::TelegramFeedRefreshTick
            | Message::TelegramFeedLoaded(_, _, _)
            | Message::TelegramAvatarLoaded(_, _, _, _)
            | Message::ToggleTelegramFastFeed
            | Message::TelegramFeedDismissOnboarding
            | Message::TelegramFeedShowOnboarding
            | Message::ToggleTelegramFastAdvanced
            | Message::TelegramFastCountryCodeChanged(_)
            | Message::TelegramFastEditNumber
            | Message::TelegramFastApiIdChanged(_)
            | Message::TelegramFastApiHashChanged(_)
            | Message::TelegramFastPhoneChanged(_)
            | Message::TelegramFastCodeChanged(_)
            | Message::TelegramFastPasswordChanged(_)
            | Message::TelegramFastRequestCode
            | Message::TelegramFastSubmitCode
            | Message::TelegramFastSubmitPassword
            | Message::TelegramFastSignOut
            | Message::TelegramFastAuthResult(_, _)
            | Message::TelegramFastFeedEvent(_, _)
            | Message::TelegramFeedChannelInputChanged(_)
            | Message::TelegramFeedAddChannel
            | Message::TelegramPrivateChannelsRefresh
            | Message::TelegramPrivateChannelsLoaded(_, _)
            | Message::TelegramFeedAddPrivateChannel(_)
            | Message::ToggleTelegramPrivateChannelCandidatesExpanded
            | Message::TelegramFeedRemoveChannel(_)
            | Message::ToggleTelegramFeedNotifications
            | Message::ToggleTelegramFeedOutcomeMarkets) => {
                return self.update_telegram_feed(message);
            }
            _ => {}
        }

        Task::none()
    }
}
