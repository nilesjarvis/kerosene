mod widgets;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Space, float, row, stack, text};
use iced::{Element, Fill, Vector};

impl TradingTerminal {
    pub(crate) fn view_tracked_trades_top_bar(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();
        let labeled_addresses = self.labeled_wallet_addresses();
        let tracked_addresses = self.tracked_trade_subscription_addresses();
        let muted_count = labeled_addresses
            .len()
            .saturating_sub(tracked_addresses.len());
        let wallet_count_label = if muted_count > 0 {
            format!(
                "{} wallets ({} muted)",
                tracked_addresses.len(),
                muted_count
            )
        } else {
            format!("{} wallets", tracked_addresses.len())
        };

        let tracked_status_label = self.tracked_trades_connection_label(now_ms);
        let tracked_status_detail = self.tracked_trades_connection_detail(now_ms);
        let status_color = Self::hydromancer_connection_color(&tracked_status_label, &theme);

        let controls = row![
            widgets::tracked_trade_connection_button(
                tracked_status_label,
                tracked_status_detail,
                status_color,
            ),
            text(wallet_count_label)
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(Fill),
            widgets::tracked_trade_settings_button(self.tracked_trade_settings_menu_open),
            widgets::tracked_trade_clear_button(),
        ]
        .spacing(8)
        .width(Fill)
        .align_y(iced::Alignment::Center);

        let mut layers: Vec<Element<'_, Message>> = vec![controls.into()];
        if self.tracked_trade_settings_menu_open {
            let dropdown_layer = float(
                row![
                    Space::new().width(Fill),
                    widgets::tracked_trade_settings_dropdown(
                        self.tracked_trade_aggregation_enabled,
                        self.tracked_trade_alerts_enabled,
                    ),
                ]
                .width(Fill)
                .align_y(iced::Alignment::Center),
            )
            .translate(|bounds, _viewport| Vector::new(0.0, bounds.height + 6.0));

            layers.push(dropdown_layer.into());
        }

        stack(layers).width(Fill).into()
    }
}
