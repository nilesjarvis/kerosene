mod assets;
mod stats;

use crate::app_state::TradingTerminal;
use crate::helpers::format_usd;
use crate::journal::AggregatedTrade;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, row, text};
use iced::{Color, Fill, Theme};

use self::stats::journal_summary_stats;

// ---------------------------------------------------------------------------
// Journal Summary
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_journal_summary<'a>(
        &'a self,
        content: Column<'a, Message>,
        filtered_trades: &[&AggregatedTrade],
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let stats = journal_summary_stats(filtered_trades);
        let total_pnl = stats.total_pnl;
        let total_fees = stats.total_fees;
        let total_closed = stats.total_closed;
        let winning_closed = stats.winning_closed;
        let win_rate = stats.win_rate();

        let pnl_color = if total_pnl > 0.0 {
            theme.palette().success
        } else if total_pnl < 0.0 {
            theme.palette().danger
        } else {
            theme.palette().text
        };

        let stat_box = |title: String, value: String, color: Color| {
            container(
                Column::new()
                    .push(
                        text(title)
                            .size(11)
                            .color(theme.extended_palette().background.weak.text),
                    )
                    .push(
                        text(value)
                            .size(18)
                            .font(iced::Font::MONOSPACE)
                            .color(color),
                    )
                    .spacing(4),
            )
            .padding([12, 16])
            .width(Fill)
            .style(move |theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                border: iced::Border {
                    color: theme.extended_palette().background.weak.color,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
        };

        let top_assets_box = self.view_journal_top_assets_box(stats.sorted_assets);

        let stats_row = row![
            stat_box(
                "Total Realized PnL".to_string(),
                format_usd(&total_pnl.to_string()),
                pnl_color
            ),
            stat_box(
                "Closed Win Rate".to_string(),
                format!("{:.1}% ({}/{})", win_rate, winning_closed, total_closed),
                theme.palette().text
            ),
            stat_box(
                "Cumulative Fees".to_string(),
                format_usd(&total_fees.to_string()),
                theme.palette().danger
            ),
        ]
        .spacing(8);

        content.push(stats_row).push(top_assets_box)
    }
}
