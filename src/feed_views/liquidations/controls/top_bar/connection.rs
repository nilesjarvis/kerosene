use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Element, Fill, Theme};

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

        tooltip(
            button(
                row![
                    status_dot,
                    text(liquidations_status_label)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text)
                        .width(Fill),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::ReconnectLiquidations)
            .padding([2, 6])
            .width(156)
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.extended_palette().background.weak.text,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
            text(liquidations_status_detail).size(10),
            iced::widget::tooltip::Position::Top,
        )
        .into()
    }
}
