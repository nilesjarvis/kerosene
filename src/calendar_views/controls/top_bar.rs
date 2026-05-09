use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_calendar_top_bar(&self, compact: bool) -> Element<'_, Message> {
        let theme = self.theme();
        let refresh_btn = button(
            text(if compact {
                "Refresh"
            } else {
                "Refresh Calendar"
            })
            .size(10)
            .center(),
        )
        .on_press(Message::RefreshCalendar)
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
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        row![
            text("Economic Calendar")
                .size(14)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::DEFAULT
                })
                .color(theme.palette().text),
            Space::new().width(Fill),
            refresh_btn,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
