mod controls;
mod daily;

use crate::account_metrics::format_signed_usd_value;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::{PnlValueDisplayMode, PortfolioPnlChart};
use iced::widget::{canvas, column, container, row, rule, scrollable, text};
use iced::{Element, Fill, color};

// ---------------------------------------------------------------------------
// Portfolio PnL View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_portfolio(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let (chart_points, value_mode, total_label, total_value, total_text, empty_chart_text) =
            if self.hide_pnl {
                let performance_points = self.selected_portfolio_performance_points();
                let total_performance = portfolio_total_performance(&performance_points);
                (
                    performance_points,
                    PnlValueDisplayMode::Percent,
                    "Total Performance:",
                    total_performance,
                    total_performance
                        .map(format_signed_percent_value)
                        .unwrap_or_else(|| "-".to_string()),
                    "No portfolio performance available",
                )
            } else {
                let pnl_points = self.selected_portfolio_points();
                let total_pnl = portfolio_total_pnl(&pnl_points);
                (
                    pnl_points,
                    PnlValueDisplayMode::Usd,
                    "Total PnL:",
                    total_pnl,
                    total_pnl
                        .map(format_signed_usd_value)
                        .unwrap_or_else(|| "-".to_string()),
                    "No portfolio history available",
                )
            };
        let skipped_invalid_points = self
            .portfolio_window_bucket()
            .map(|bucket| bucket.skipped_invalid_points)
            .unwrap_or_default();

        let total_value_color = match total_value {
            Some(value) if value >= 0.0 => theme.palette().success,
            Some(_) => theme.palette().danger,
            None => theme.extended_palette().background.weak.text,
        };

        let chart_body: Element<'_, Message> = if chart_points.len() >= 2 {
            canvas(PortfolioPnlChart {
                points: chart_points.clone(),
                value_mode,
            })
            .width(Fill)
            .height(220)
            .into()
        } else if self.portfolio.loading {
            self.loading_overlay("Loading portfolio...")
        } else {
            container(
                text(empty_chart_text)
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(220)
            .center(Fill)
            .into()
        };

        let mut content = column![
            self.view_portfolio_title(),
            self.view_portfolio_window_controls(),
            row![
                text(total_label).size(11).color(theme.palette().text),
                text(total_text).size(14).color(total_value_color),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            chart_body,
            rule::horizontal(1),
            text(if self.hide_pnl {
                "Daily Performance (last 7 days)"
            } else {
                "Daily PnL (last 7 days)"
            })
            .size(11)
            .color(theme.palette().text),
            text("Source: allTime/perpAllTime (UTC days)")
                .size(10)
                .color(color!(0x7f8ab0)),
            self.view_daily_pnl_list(&theme),
        ]
        .spacing(8);

        if skipped_invalid_points > 0 {
            content = content.push(
                text(format!(
                    "Invalid portfolio history points skipped: {skipped_invalid_points}"
                ))
                .size(11)
                .color(theme.palette().danger),
            );
        }

        if let Some(err) = &self.portfolio.last_error {
            content = content.push(
                text(format!("Stale data: {err}"))
                    .size(11)
                    .color(theme.palette().primary),
            );
        }

        let scroll = scrollable(container(content).width(Fill).padding(iced::Padding {
            top: 0.0,
            right: 15.0,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(4.0)
                .scroller_width(4.0)
                .margin(0.0),
        ))
        .height(Fill);

        container(scroll)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }
}

fn portfolio_total_performance(points: &[(u64, f64)]) -> Option<f64> {
    points
        .last()
        .and_then(|(_, value)| value.is_finite().then_some(*value))
}

fn portfolio_total_pnl(points: &[(u64, f64)]) -> Option<f64> {
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

fn format_signed_percent_value(value: f64) -> String {
    let display_value = if value.abs() < 0.005 { 0.0 } else { value };
    if display_value > 0.0 {
        format!("+{display_value:.2}%")
    } else {
        format!("{display_value:.2}%")
    }
}

#[cfg(test)]
mod tests {
    use super::{format_signed_percent_value, portfolio_total_performance, portfolio_total_pnl};

    #[test]
    fn portfolio_total_pnl_is_unknown_without_points_or_invalid_values() {
        assert_eq!(portfolio_total_pnl(&[]), None);
        assert_eq!(portfolio_total_pnl(&[(1, f64::NAN)]), None);
        assert_eq!(portfolio_total_pnl(&[(1, 1.0), (2, f64::INFINITY)]), None);
    }

    #[test]
    fn portfolio_total_pnl_uses_single_value_or_first_last_delta() {
        assert_eq!(portfolio_total_pnl(&[(1, 5.0)]), Some(5.0));
        assert_eq!(portfolio_total_pnl(&[(1, 5.0), (2, 12.0)]), Some(7.0));
    }

    #[test]
    fn portfolio_total_performance_uses_latest_percent_value() {
        assert_eq!(
            portfolio_total_performance(&[(1, 0.0), (2, 1.5)]),
            Some(1.5)
        );
        assert_eq!(portfolio_total_performance(&[(1, f64::NAN)]), None);
    }

    #[test]
    fn format_signed_percent_value_marks_positive_values() {
        assert_eq!(format_signed_percent_value(1.234), "+1.23%");
        assert_eq!(format_signed_percent_value(-1.234), "-1.23%");
        assert_eq!(format_signed_percent_value(0.001), "0.00%");
    }
}
