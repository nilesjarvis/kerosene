mod widgets;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Space, row, text, tooltip};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_tracked_trades_top_bar(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();
        let labeled_addresses = self.labeled_wallet_addresses();

        let tracked_status_label = self.tracked_trades_connection_label(now_ms);
        let tracked_status_detail = self.tracked_trades_connection_detail(now_ms);
        let status_color = Self::hydromancer_connection_color(&tracked_status_label, &theme);

        let aggregation_btn = widgets::tracked_trade_toggle_button(
            if self.tracked_trade_aggregation_enabled {
                "Rows: Orders"
            } else {
                "Rows: Fills"
            },
            self.tracked_trade_aggregation_enabled,
            true,
            Message::ToggleTrackedTradeAggregation,
        );
        let alerts_btn = widgets::tracked_trade_toggle_button(
            if self.tracked_trade_alerts_enabled {
                "Alerts: ON"
            } else {
                "Alerts: OFF"
            },
            self.tracked_trade_alerts_enabled,
            false,
            Message::ToggleTrackedTradeAlerts,
        );

        row![
            widgets::tracked_trade_status_dot(status_color),
            tooltip(
                text(tracked_status_label)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
                    .width(130),
                text(tracked_status_detail).size(10),
                iced::widget::tooltip::Position::Top,
            ),
            widgets::tracked_trade_reconnect_button(),
            text(format!("{} wallets", labeled_addresses.len()))
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(Fill),
            aggregation_btn,
            alerts_btn,
            widgets::tracked_trade_clear_button(),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
