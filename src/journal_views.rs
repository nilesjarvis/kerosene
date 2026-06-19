mod header;
mod status;
mod style;
mod summary;
mod trade_card;
mod trades;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, container, scrollable, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Trading journal view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_journal(&self) -> Element<'_, Message> {
        let theme = self.theme();

        let mut content = Column::new()
            .spacing(12)
            .push(self.view_journal_header())
            .padding(20);

        content = self.push_journal_warning(content, &theme);
        let (visible_fill_count, visible_trade_count) = self.journal_visible_counts();
        content =
            self.push_journal_status(content, visible_fill_count, visible_trade_count, &theme);

        if self.journal.loading && visible_trade_count == 0 {
            content = content.push(
                text("Loading trades...")
                    .size(14)
                    .color(theme.palette().success),
            );
        } else if let Some(e) = &self.journal.error {
            content = content.push(
                text(format!("Error: {}", e))
                    .size(14)
                    .color(theme.palette().danger),
            );
        } else if visible_trade_count == 0 {
            content = content.push(
                text("No trades found.")
                    .size(14)
                    .color(theme.palette().success),
            );
        } else {
            let filtered_trades = self.filtered_journal_trades();

            content = self.push_journal_summary(content, &filtered_trades);

            if filtered_trades.is_empty() {
                content = content.push(
                    text("No trades match the current filter.")
                        .size(14)
                        .color(theme.palette().text),
                );
            } else {
                let current_time_ms = self.status_bar_now_ms;
                let trade_list = if self.journal_trades_view.is_table() {
                    self.view_journal_trade_table(&filtered_trades, current_time_ms)
                } else {
                    let mut list = Column::new().spacing(8);

                    for trade in filtered_trades {
                        list = list.push(self.view_journal_trade_card(trade, current_time_ms));
                    }

                    if self.journal.loading {
                        list = list.push(self.view_journal_fetching_history_row(&theme));
                    }

                    list.into()
                };

                content = content.push(
                    scrollable(trade_list)
                        .direction(iced::widget::scrollable::Direction::Vertical(
                            iced::widget::scrollable::Scrollbar::new()
                                .width(4)
                                .margin(0)
                                .scroller_width(4),
                        ))
                        .width(Fill)
                        .height(Fill),
                );
            }
        }

        container(content)
            .width(Fill)
            .height(Fill)
            .style(|t: &Theme| container_style::Style {
                background: Some(t.palette().background.into()),
                ..Default::default()
            })
            .into()
    }
}
