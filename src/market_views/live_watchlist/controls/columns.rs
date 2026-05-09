use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::LiveWatchlistId;
use crate::message::Message;
use iced::widget::{checkbox, row, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_column_controls(
        &self,
        id: LiveWatchlistId,
        visible_columns: &[config::LiveWatchlistColumn],
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let mut column_controls = row![
            text("Columns")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        for column in config::LiveWatchlistColumn::ALL {
            let enabled = visible_columns.contains(&column);
            column_controls = column_controls.push(
                checkbox(enabled)
                    .label(column.label())
                    .on_toggle(move |checked| {
                        Message::LiveWatchlistColumnToggled(id, column, checked)
                    })
                    .size(12)
                    .text_size(10),
            );
        }

        scrollable(column_controls)
            .direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::new()
                    .width(0)
                    .margin(0)
                    .scroller_width(0),
            ))
            .width(Fill)
            .into()
    }
}
