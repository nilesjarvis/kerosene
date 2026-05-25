use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::{
    format_decimal_with_commas, normalize_two_decimal_display_value, signed_number_color,
};
use crate::journal::{AggregatedTrade, JournalFilter};
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, canvas, checkbox, column, container, row, rule, text};
use iced::{Alignment, Element, Fill, Theme};

mod drawing;
mod outcome;
mod series;

use drawing::JournalSummaryChart;
use outcome::*;
use series::*;

// ---------------------------------------------------------------------------
// Journal Summary Chart
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_journal_summary_chart(
        &self,
        filtered_trades: &[&AggregatedTrade],
        total_pnl: f64,
        total_fees: f64,
        win_rate: f64,
        total_closed: usize,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let pnl_points = journal_cumulative_pnl_points(filtered_trades);
        let account_value_points = self.journal_account_value_chart_points(&pnl_points);
        let show_account_value = self.journal.show_account_value_chart;
        let denomination = self.display_denomination_context();

        let value_color = signed_number_color(total_pnl, &theme);
        let muted = theme.extended_palette().background.weak.text;
        let filter_label = journal_filter_label(self.journal.filter);
        let win_rate_color = if total_closed == 0 {
            muted
        } else if win_rate >= 50.0 {
            theme.palette().success
        } else {
            theme.palette().danger
        };

        let chart_body: Element<'_, Message> = if pnl_points.len() >= 2 {
            canvas(JournalSummaryChart {
                pnl_points,
                account_value_points,
                show_account_value,
                denomination: denomination.clone(),
            })
            .width(Fill)
            .height(112)
            .into()
        } else {
            container(
                text("No PnL history available")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(112)
            .center(Fill)
            .into()
        };

        let account_toggle = checkbox(show_account_value)
            .label("Acct value")
            .on_toggle(Message::JournalToggleAccountValueChart)
            .size(10)
            .spacing(4)
            .text_size(10)
            .font(crate::app_fonts::monospace_font());

        let content = column![
            row![
                text("Performance").size(14).color(theme.palette().text),
                Space::new().width(Fill),
                text(filter_label).size(12).color(muted),
                account_toggle,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            rule::horizontal(1),
            row![
                text(format_signed_display_full(total_pnl, &denomination))
                    .size(22)
                    .font(crate::app_fonts::monospace_font())
                    .color(value_color),
                text("PNL")
                    .size(13)
                    .font(crate::app_fonts::monospace_font())
                    .color(muted),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            chart_body,
            column![
                journal_outcome_strip(filtered_trades, denomination.clone()),
                column![
                    text(format!("{win_rate:.1}% Win Rate"))
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(win_rate_color),
                    text(trade_count_label(total_closed))
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(muted),
                    text(format!("Fees {}", denomination.format_value(total_fees, 2)))
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(theme.palette().danger),
                ]
                .spacing(2)
                .align_x(Alignment::Start),
            ]
            .spacing(3)
            .align_x(Alignment::Start),
        ]
        .spacing(7)
        .height(Fill);

        container(content)
            .padding([12, 16])
            .width(Fill)
            .height(320)
            .style(|theme: &Theme| summary_panel_style(theme))
            .into()
    }

    fn journal_account_value_chart_points(&self, pnl_points: &[(u64, f64)]) -> Vec<(u64, f64)> {
        if !self.journal.show_account_value_chart {
            return Vec::new();
        }

        let Some((start_ms, end_ms)) = chart_time_range(pnl_points) else {
            return Vec::new();
        };
        let bucket_key = match self.journal.filter {
            JournalFilter::Perp => "perpAllTime",
            JournalFilter::All | JournalFilter::Spot => "allTime",
        };

        self.portfolio_bucket_by_key(bucket_key)
            .or_else(|| self.portfolio_bucket_by_key("allTime"))
            .map(|bucket| {
                account_value_points_for_range(&bucket.account_value_history, start_ms, end_ms)
            })
            .unwrap_or_default()
    }
}

fn journal_filter_label(filter: JournalFilter) -> &'static str {
    match filter {
        JournalFilter::All => "All",
        JournalFilter::Perp => "Perp",
        JournalFilter::Spot => "Spot",
    }
}

#[cfg(test)]
fn format_signed_usd_full(value: f64) -> String {
    format_signed_display_full(value, &DisplayDenominationContext::default())
}

fn format_signed_display_full(value: f64, denomination: &DisplayDenominationContext) -> String {
    let display_value = normalize_two_decimal_display_value(value);
    if denomination.active_code() != "USD" {
        return denomination.format_signed_value(display_value, 2);
    }
    let sign = if display_value > 0.0 {
        "+"
    } else if display_value < 0.0 {
        "-"
    } else {
        ""
    };
    format!(
        "{sign}${}",
        format_decimal_with_commas(display_value.abs(), 2)
    )
}

fn trade_count_label(total_closed: usize) -> String {
    if total_closed == 1 {
        "1 Trade".to_string()
    } else {
        format!("{total_closed} Trades")
    }
}

fn summary_panel_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        border: iced::Border {
            color: theme.extended_palette().background.weak.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
