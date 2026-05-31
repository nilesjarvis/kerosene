mod controls;
mod daily;
mod totals;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::{PnlValueDisplayMode, PortfolioPnlChart};
use iced::widget::{canvas, column, container, row, rule, scrollable, text};
use iced::{Element, Fill};
use totals::{format_signed_percent_value, portfolio_total_performance, portfolio_total_pnl};

// ---------------------------------------------------------------------------
// Portfolio PnL View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_portfolio(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let value_mode = self.portfolio_pnl_value_display_mode();
        let (chart_points, total_label, total_value, total_text, empty_chart_text) =
            if value_mode == PnlValueDisplayMode::Percent {
                let performance_points = self.selected_portfolio_performance_points();
                let total_performance = portfolio_total_performance(&performance_points);
                (
                    performance_points,
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
                let denomination = self.display_denomination_context();
                (
                    pnl_points,
                    "Total PnL:",
                    total_pnl,
                    total_pnl
                        .map(|value| denomination.format_signed_value(value, 2))
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
                denomination: self.display_denomination_context(),
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
            text(if value_mode == PnlValueDisplayMode::Percent {
                "Daily Performance (last 7 days)"
            } else {
                "Daily PnL (last 7 days)"
            })
            .size(11)
            .color(theme.palette().text),
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

    fn portfolio_pnl_value_display_mode(&self) -> PnlValueDisplayMode {
        self.portfolio.pnl_value_display_mode
    }
}

#[cfg(test)]
mod tests;
