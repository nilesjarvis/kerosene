use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::{
    format_decimal_with_commas, normalize_two_decimal_display_value, signed_number_color,
};
use crate::journal::{AggregatedTrade, JournalFilter};
use crate::journal_views::style::{JOURNAL_PANEL_PADDING, journal_panel_style};
use crate::message::Message;
use crate::portfolio_state::PortfolioWindow;

use iced::widget::{Space, button, canvas, checkbox, column, container, row, rule, text};
use iced::{Alignment, Element, Fill};

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
    pub(in crate::journal_views) fn view_journal_summary_chart(
        &self,
        filtered_trades: &[&AggregatedTrade],
        total_pnl: f64,
        total_fees: f64,
        win_rate: f64,
        total_closed: usize,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let trade_pnl_points =
            journal_cumulative_pnl_points(filtered_trades, self.journal.include_fees_in_pnl);
        let portfolio_pnl_points = self.journal_portfolio_margin_pnl_points();
        let using_portfolio_pnl = portfolio_pnl_points.is_some();
        let all_pnl_points = portfolio_pnl_points.unwrap_or(trade_pnl_points);
        let pnl_points = self.journal_portfolio_window_points(all_pnl_points);
        let selected_window_pnl = journal_window_total_pnl(&pnl_points).unwrap_or(total_pnl);
        let account_value_points = self.journal_account_value_chart_points(&pnl_points);
        let show_account_value = self.journal.show_account_value_chart;
        let denomination = self.display_denomination_context();

        let value_color = signed_number_color(selected_window_pnl, &theme);
        let muted = theme.extended_palette().background.weak.text;
        let filter_label = journal_filter_label(self.journal.filter);
        let pnl_label = if using_portfolio_pnl {
            "PORTFOLIO PNL"
        } else if self.journal.include_fees_in_pnl {
            "NET PNL"
        } else {
            "GROSS PNL"
        };
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
                reveal_progress: self.journal.chart_reveal_progress,
            })
            .width(Fill)
            .height(136)
            .into()
        } else {
            container(
                text("No PnL history available")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(136)
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
                button(
                    text(format_signed_display_full(
                        selected_window_pnl,
                        &denomination
                    ))
                    .size(22)
                    .font(crate::app_fonts::monospace_font())
                    .color(value_color)
                )
                .on_press(Message::JournalToggleIncludeFeesInPnl)
                .padding(0)
                .style(button::text),
                text(pnl_label)
                    .size(13)
                    .font(crate::app_fonts::monospace_font())
                    .color(muted),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            chart_body,
            column![
                journal_outcome_strip(
                    filtered_trades,
                    denomination.clone(),
                    self.journal.include_fees_in_pnl
                ),
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
        .spacing(6)
        .height(Fill);

        container(content)
            .padding(JOURNAL_PANEL_PADDING)
            .width(Fill)
            .height(372)
            .style(journal_panel_style)
            .into()
    }

    fn journal_portfolio_window_points(&self, points: Vec<(u64, f64)>) -> Vec<(u64, f64)> {
        apply_journal_portfolio_window(
            points,
            self.journal.portfolio_window,
            self.status_bar_now_ms,
        )
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
            JournalFilter::All | JournalFilter::Spot | JournalFilter::Outcome => "allTime",
        };

        self.portfolio_bucket_by_key(bucket_key)
            .or_else(|| self.portfolio_bucket_by_key("allTime"))
            .map(|bucket| {
                account_value_points_for_range(&bucket.account_value_history, start_ms, end_ms)
            })
            .unwrap_or_default()
    }

    fn journal_portfolio_margin_pnl_points(&self) -> Option<Vec<(u64, f64)>> {
        let is_portfolio_margin = self
            .connected_order_account_snapshot()
            .is_some_and(|(_, data)| data.is_portfolio_margin());
        if !is_portfolio_margin {
            return None;
        }

        let kind = journal_portfolio_pnl_kind(self.journal.filter)?;
        let points = match kind {
            JournalPortfolioPnlKind::All | JournalPortfolioPnlKind::Perp => {
                self.journal_portfolio_bucket_pnl_points(kind)?
            }
            JournalPortfolioPnlKind::NonPerp => {
                let all_points =
                    self.journal_portfolio_bucket_pnl_points(JournalPortfolioPnlKind::All)?;
                let perp_points = self
                    .journal_portfolio_bucket_pnl_points(JournalPortfolioPnlKind::Perp)
                    .unwrap_or_default();
                subtract_latest_pnl_series(&all_points, &perp_points)
            }
        };

        (!points.is_empty()).then_some(points)
    }

    fn journal_portfolio_bucket_pnl_points(
        &self,
        kind: JournalPortfolioPnlKind,
    ) -> Option<Vec<(u64, f64)>> {
        if let Some(key) =
            journal_direct_portfolio_pnl_bucket_key(kind, self.journal.portfolio_window)
            && let Some(bucket) = self.portfolio_bucket_by_key(key)
            && !bucket.pnl_history.is_empty()
        {
            return Some(bucket.pnl_history.clone());
        }

        let all_time_key = journal_all_time_portfolio_pnl_bucket_key(kind)?;
        self.portfolio_bucket_by_key(all_time_key)
            .map(|bucket| bucket.pnl_history.clone())
            .filter(|points| !points.is_empty())
    }
}

fn journal_direct_portfolio_pnl_bucket_key(
    kind: JournalPortfolioPnlKind,
    window: PortfolioWindow,
) -> Option<&'static str> {
    match (kind, window) {
        (JournalPortfolioPnlKind::All, PortfolioWindow::Day) => Some("day"),
        (JournalPortfolioPnlKind::All, PortfolioWindow::Week) => Some("week"),
        (JournalPortfolioPnlKind::All, PortfolioWindow::Month) => Some("month"),
        (JournalPortfolioPnlKind::Perp, PortfolioWindow::Day) => Some("perpDay"),
        (JournalPortfolioPnlKind::Perp, PortfolioWindow::Week) => Some("perpWeek"),
        (JournalPortfolioPnlKind::Perp, PortfolioWindow::Month) => Some("perpMonth"),
        _ => None,
    }
}

fn journal_all_time_portfolio_pnl_bucket_key(
    kind: JournalPortfolioPnlKind,
) -> Option<&'static str> {
    match kind {
        JournalPortfolioPnlKind::All => Some("allTime"),
        JournalPortfolioPnlKind::Perp => Some("perpAllTime"),
        JournalPortfolioPnlKind::NonPerp => None,
    }
}

fn journal_filter_label(filter: JournalFilter) -> &'static str {
    match filter {
        JournalFilter::All => "All",
        JournalFilter::Perp => "Perp",
        JournalFilter::Spot => "Spot",
        JournalFilter::Outcome => "Outcome",
    }
}

fn apply_journal_portfolio_window(
    points: Vec<(u64, f64)>,
    window: PortfolioWindow,
    now_ms: u64,
) -> Vec<(u64, f64)> {
    if points.is_empty() || matches!(window, PortfolioWindow::AllTime) {
        return points;
    }

    let Some(cutoff) = window.cutoff_ms(now_ms) else {
        return points;
    };
    TradingTerminal::apply_cutoff_with_baseline(&points, cutoff)
}

fn journal_window_total_pnl(points: &[(u64, f64)]) -> Option<f64> {
    match points {
        [] => None,
        [(_, only)] => only.is_finite().then_some(*only),
        points => {
            let first = points.first().map(|(_, value)| *value)?;
            let last = points.last().map(|(_, value)| *value)?;
            let total = last - first;
            total.is_finite().then_some(total)
        }
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

#[cfg(test)]
mod tests;
