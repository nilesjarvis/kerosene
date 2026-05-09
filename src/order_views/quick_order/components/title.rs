use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use iced::widget::{Row, button, row, text};
use iced::{Color, Theme};

impl TradingTerminal {
    pub(in crate::order_views::quick_order) fn quick_order_title_row<'a>(
        chart_id: ChartId,
        form: &QuickOrderForm,
        type_label: String,
    ) -> Row<'a, Message> {
        let type_toggle = button(
            text(if form.is_limit { "Limit" } else { "Market" })
                .size(10)
                .center(),
        )
        .on_press(Message::QuickOrderToggleType(chart_id))
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        row![text(type_label).size(12).color(Color::WHITE), type_toggle]
            .spacing(8)
            .align_y(iced::Alignment::Center)
    }
}
