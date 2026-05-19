use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, pane_grid, text, tooltip};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(crate) fn view_chart_add_button(&self, pane: pane_grid::Pane) -> Element<'static, Message> {
        tooltip(
            button(text("+").size(11).center())
                .on_press(Message::AddChart(pane))
                .padding([2, 6])
                .style(|theme: &Theme, status| {
                    let bg = match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => theme.extended_palette().background.weak.color,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: theme.palette().success,
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
            text("Add candlestick chart")
                .size(10)
                .font(iced::Font::MONOSPACE),
            tooltip::Position::Bottom,
        )
        .into()
    }
}
