mod actions;
mod identity;
mod metrics;
mod status;

use self::actions::wallet_tracker_actions;
use self::identity::wallet_identity_cell;
use self::metrics::{money_text, wallet_row_metrics, wallet_upnl_color};

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_wallet_tracker_row<'a>(
        &'a self,
        address: &str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let row_data = self
            .wallet_tracker
            .rows
            .get(address)
            .cloned()
            .unwrap_or_default();
        let display = self.wallet_display(address);
        let label_value = self.wallet_label(address).unwrap_or_default().to_string();
        let denomination = self.display_denomination_context();
        let metrics = wallet_row_metrics(&row_data, &denomination, theme);
        let upnl_color = wallet_upnl_color(&metrics, theme);
        let state_el: Element<'_, Message> = self.view_wallet_tracker_state(&row_data, theme);

        let address = address.to_string();
        let wallet_row = container(
            row![
                wallet_identity_cell(
                    address.clone(),
                    label_value,
                    display,
                    self.hovered_wallet_address_actions.as_deref(),
                    theme,
                ),
                money_text(metrics.equity, metrics.data_color).width(85),
                money_text(metrics.withdrawable, metrics.data_color).width(85),
                text(metrics.upnl)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(upnl_color)
                    .width(75),
                text(metrics.margin)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(metrics.data_color)
                    .width(60),
                text(metrics.risk)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(metrics.data_color)
                    .width(95),
                container(state_el).width(90),
                Space::new().width(Fill),
                wallet_tracker_actions(address.clone(), self.wallet_tracker.is_muted(&address)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding([8, 8])
        .style(|theme: &Theme| container_style::Style {
            background: Some(
                Color {
                    a: 0.35,
                    ..theme.extended_palette().background.weak.color
                }
                .into(),
            ),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        });

        wallet_row.into()
    }
}
