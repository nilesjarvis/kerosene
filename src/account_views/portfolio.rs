mod controls;
mod daily;

use crate::account_metrics::format_signed_usd_value;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::PortfolioPnlChart;
use iced::widget::{canvas, column, container, row, rule, scrollable, text};
use iced::{Element, Fill, color};

// ---------------------------------------------------------------------------
// Portfolio PnL View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_portfolio(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let chart_points: Vec<(u64, f64)> = self.selected_portfolio_points();
        let skipped_invalid_points = self
            .portfolio_window_bucket()
            .map(|bucket| bucket.skipped_invalid_points)
            .unwrap_or_default();

        let total_pnl = portfolio_total_pnl(&chart_points);
        let total_pnl_color = match total_pnl {
            Some(value) if value >= 0.0 => theme.palette().success,
            Some(_) => theme.palette().danger,
            None => theme.extended_palette().background.weak.text,
        };
        let total_pnl_text = total_pnl
            .map(format_signed_usd_value)
            .unwrap_or_else(|| "-".to_string());

        let chart_body: Element<'_, Message> = if chart_points.len() >= 2 {
            canvas(PortfolioPnlChart {
                points: chart_points.clone(),
            })
            .width(Fill)
            .height(220)
            .into()
        } else if self.portfolio.loading {
            self.loading_overlay("Loading portfolio...")
        } else {
            container(
                text("No portfolio history available")
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
                text("Total PnL:").size(11).color(theme.palette().text),
                text(total_pnl_text).size(14).color(total_pnl_color),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            chart_body,
            rule::horizontal(1),
            text("Daily PnL (last 7 days)")
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

#[cfg(test)]
mod tests {
    use super::portfolio_total_pnl;

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
}
