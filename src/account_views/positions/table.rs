use crate::account;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::PositionColumnVisibility;
use iced::Theme;
use iced::widget::Column;

mod close_cell;
mod position_row;
mod sort;
mod summary;

impl TradingTerminal {
    pub(super) fn view_position_rows<'a>(
        &'a self,
        positions: &[&'a account::AssetPosition],
        can_close: bool,
        theme: &Theme,
        columns: PositionColumnVisibility,
    ) -> Column<'a, Message> {
        self.sorted_position_rows(positions)
            .into_iter()
            .fold(Column::new().spacing(2), |col, data| {
                col.push(self.view_position_row(data, can_close, theme, columns))
            })
    }
}
