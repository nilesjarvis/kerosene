use crate::denomination::DisplayDenominationContext;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme, color};

#[allow(clippy::too_many_arguments)]
pub(in crate::journal_views::trade_card) fn journal_trade_card_header(
    display_coin: String,
    status: String,
    opened_time_str: String,
    max_position_label: String,
    pnl: f64,
    status_color: Color,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
) -> Element<'static, Message> {
    row![
        text(display_coin).size(16),
        Space::new().width(8.0),
        container(text(status).size(10).color(theme.palette().background))
            .padding([2, 6])
            .style(move |_theme: &Theme| container_style::Style {
                background: Some(status_color.into()),
                border: iced::Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        Space::new().width(12.0),
        text(opened_time_str).size(12).color(color!(0x888888)),
        Space::new().width(Fill),
        text(format!("Max Pos: {}", max_position_label))
            .font(crate::app_fonts::monospace_font())
            .size(12)
            .color(theme.palette().text),
        Space::new().width(12.0),
        text(format!("PnL: {}", denomination.format_value(pnl, 2)))
            .size(12)
            .color(if pnl > 0.0 {
                theme.palette().success
            } else if pnl < 0.0 {
                theme.palette().danger
            } else {
                theme.palette().text
            }),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
