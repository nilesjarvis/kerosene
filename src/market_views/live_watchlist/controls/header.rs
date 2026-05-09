use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::{LiveWatchlistId, LiveWatchlistInstance};
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_header(
        &self,
        id: LiveWatchlistId,
        wl: &LiveWatchlistInstance,
        display_columns: &[config::LiveWatchlistColumn],
    ) -> Element<'static, Message> {
        let theme = self.theme();
        let sort_column = wl.sort_column;
        let sort_direction = wl.sort_direction;
        let sort_btn = |label: &'static str, col: config::LiveWatchlistSortColumn, width: f32| {
            let mut row_content = row![
                text(label)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
            ];
            if sort_column == col {
                let icon = if sort_direction == config::SortDirection::Ascending {
                    "\u{2191}"
                } else {
                    "\u{2193}"
                };
                row_content = row_content.push(
                    text(icon)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                );
            }
            button(row_content.spacing(2))
                .on_press(Message::LiveWatchlistSortChanged(id, col))
                .style(|_t: &Theme, _| button::Style {
                    background: None,
                    ..Default::default()
                })
                .padding(0)
                .width(if width == 0.0 {
                    iced::Length::Fill
                } else {
                    iced::Length::Fixed(width)
                })
        };

        let mut header = row![sort_btn(
            "Symbol",
            config::LiveWatchlistSortColumn::Symbol,
            0.0
        )];
        for column in display_columns {
            header = header.push(sort_btn(
                column.label(),
                column.sort_column(),
                column.width(),
            ));
        }
        header
            .push(Space::new().width(20))
            .spacing(8)
            .padding([4, 8])
            .into()
    }
}
