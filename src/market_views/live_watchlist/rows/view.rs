mod cells;
mod flash;

use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::{LiveWatchlistId, LiveWatchlistRowData};
use crate::message::Message;

use super::super::symbol_cell::view_live_watchlist_symbol_cell;
use iced::widget::row;
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_row<'a>(
        &self,
        id: LiveWatchlistId,
        data: &'a LiveWatchlistRowData,
        display_columns: &[config::LiveWatchlistColumn],
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let price_color = self.live_watchlist_price_color(&data.sym_key, now_ms, theme);
        let denomination = self.display_denomination_context();

        let mut row_content = row![
            view_live_watchlist_symbol_cell(
                &data.sym_key,
                &data.display,
                self.exchange_symbol_for_key(&data.sym_key)
                    .is_some_and(|symbol| symbol.growth_mode),
                theme,
            )
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
