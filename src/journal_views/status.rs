use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::journal_views::style::journal_accent_mint;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, container, row, text};
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(super) fn journal_visible_counts(&self) -> (usize, usize) {
        let visible_fill_count = self
            .journal
            .raw_fills
            .iter()
            .filter(|fill| !self.symbol_key_is_hidden(&fill.coin))
            .count();
        let visible_trade_count = self
            .journal
            .trades
            .iter()
            .filter(|trade| !self.symbol_key_is_hidden(&trade.coin))
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
        let muted = theme.extended_palette().background.weak.text;
        let has_counts = visible_fill_count > 0 || visible_trade_count > 0;
        let is_syncing = self.journal.loading && visible_trade_count > 0;

        if !has_counts && self.journal.last_refresh_time.is_none() && !is_syncing {
            return content;
        }

        let mut status_row = row![].spacing(8).align_y(iced::Alignment::Center);

        if has_counts {
            status_row = status_row.push(
                text(format!(
                    "{} fills  |  {} trades",
                    visible_fill_count, visible_trade_count
                ))
                .size(11)
                .color(muted),
            );
        }

        if let Some(last_refresh_time) = self.journal.last_refresh_time {
            status_row = status_row.push(
                text(format!(
                    "Synced {}",
                    helpers::format_timestamp_exact(last_refresh_time)
                ))
                .size(11)
                .color(theme.palette().success),
            );
        }

        status_row = status_row.push(Space::new().width(Fill));

        if is_syncing {
            status_row = status_row.push(
                text("Syncing history...")
                    .size(11)
                    .color(journal_accent_mint()),
            );
        }

        content.push(status_row)
    }
}
