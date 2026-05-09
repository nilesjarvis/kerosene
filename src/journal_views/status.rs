use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, text};
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(super) fn journal_visible_counts(&self) -> (usize, usize) {
        let visible_fill_count = self
            .journal
            .raw_fills
            .iter()
            .filter(|fill| !self.is_ticker_muted(&fill.coin))
            .count();
        let visible_trade_count = self
            .journal
            .trades
            .iter()
            .filter(|trade| !self.is_ticker_muted(&trade.coin))
            .count();

        (visible_fill_count, visible_trade_count)
    }

    pub(super) fn push_journal_warning<'a>(
        &'a self,
        content: Column<'a, Message>,
        theme: &Theme,
    ) -> Column<'a, Message> {
        if let Some(warning) = &self.journal.warning {
            content.push(
                container(text(warning).size(12).color(theme.palette().text))
                    .width(Fill)
                    .padding([8, 12])
                    .style(|t: &Theme| container_style::Style {
                        background: Some(t.extended_palette().background.weak.color.into()),
                        border: iced::Border {
                            color: t.palette().primary,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }),
            )
        } else {
            content
        }
    }

    pub(super) fn push_journal_status<'a>(
        &self,
        content: Column<'a, Message>,
        visible_fill_count: usize,
        visible_trade_count: usize,
        theme: &Theme,
    ) -> Column<'a, Message> {
        let mut status_parts = Vec::new();
        if visible_fill_count > 0 || visible_trade_count > 0 {
            status_parts.push(format!("{} fills", visible_fill_count));
            status_parts.push(format!("{} trades", visible_trade_count));
        }
        if let Some(last_refresh_time) = self.journal.last_refresh_time {
            status_parts.push(format!(
                "Synced {}",
                helpers::format_timestamp_exact(last_refresh_time)
            ));
        }
        if self.journal.loading && visible_trade_count > 0 {
            status_parts.push("Syncing history".to_string());
        }

        if status_parts.is_empty() {
            content
        } else {
            content.push(
                text(status_parts.join("  |  "))
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            )
        }
    }
}
