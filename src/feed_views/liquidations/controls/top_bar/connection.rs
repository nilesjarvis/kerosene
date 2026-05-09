use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(in crate::feed_views::liquidations::controls) fn view_liquidations_connection_controls(
        &self,
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let liquidations_status_label = self.liquidations_connection_label(now_ms);
        let liquidations_status_detail = self.liquidations_connection_detail(now_ms);
        let status_color = Self::hydromancer_connection_color(&liquidations_status_label, theme);
        let status_dot =
            container(Space::new().width(8.0).height(8.0)).style(move |_| container_style::Style {
                background: Some(status_color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        row![
            status_dot,
            tooltip(
                text(liquidations_status_label)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
                    .width(130),
                text(liquidations_status_detail).size(10),
                iced::widget::tooltip::Position::Top,
            ),
            reconnect_liquidations_button(),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn reconnect_liquidations_button() -> button::Button<'static, Message> {
    button(text("Reconnect").size(10))
        .on_press(Message::ReconnectLiquidations)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().primary,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
}
