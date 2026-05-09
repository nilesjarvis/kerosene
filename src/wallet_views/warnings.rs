use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, container, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Wallet Detail Warnings
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_wallet_detail_warnings<'a>(
        &self,
        warnings: &'a [String],
        theme: &Theme,
    ) -> Option<Element<'a, Message>> {
        if warnings.is_empty() {
            return None;
        }

        let warning_rows = warnings.iter().fold(
            Column::new()
                .spacing(4)
                .push(text("Warnings").size(13).color(theme.palette().danger)),
            |column, warning| {
                column.push(
                    text(warning.clone())
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                )
            },
        );
        Some(container(warning_rows).padding([6, 8]).width(Fill).into())
    }
}
