mod cells;
mod flash;

use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::{LiveWatchlistId, LiveWatchlistRowData};
use crate::message::Message;

use iced::widget::row;
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_row(
        &self,
        id: LiveWatchlistId,
        data: &LiveWatchlistRowData,
        display_columns: &[config::LiveWatchlistColumn],
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let price_color = self.live_watchlist_price_color(&data.sym_key, now_ms, theme);
        let denomination = self.display_denomination_context();

        let mut row_content = row![
            cells::live_watchlist_symbol_cell(&data.sym_key, data.display.clone(), theme)
                .width(Fill)
        ];
        for column in display_columns {
            let (value, color) =
                cells::live_watchlist_column_value(column, data, &denomination, price_color, theme);
            row_content = row_content.push(cells::live_watchlist_column_cell(column, value, color));
        }
        row_content = row_content
            .push(cells::live_watchlist_remove_button(
                id,
                data.sym_key.clone(),
                theme,
            ))
            .spacing(8)
            .align_y(iced::Alignment::Center);

        cells::live_watchlist_row_button(data.sym_key.clone(), row_content)
    }
}
