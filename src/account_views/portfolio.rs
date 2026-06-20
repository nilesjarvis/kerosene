mod controls;
mod daily;
mod header;
mod tokens;
mod totals;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::{PnlValueDisplayMode, PortfolioPnlChart, PortfolioScope};
use header::view_portfolio_hero;
use iced::widget::{canvas, column, container, scrollable, text};
use iced::{Element, Fill};
use totals::{format_signed_percent_value, portfolio_total_performance, portfolio_total_pnl};

// ---------------------------------------------------------------------------
// Portfolio Pane
// ---------------------------------------------------------------------------

const CHART_HEIGHT: f32 = 160.0;
const BLOCK_GAP: f32 = 14.0;
const BODY_PADDING: f32 = 16.0;

impl TradingTerminal {
    pub(crate) fn view_portfolio(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let value_mode = self.portfolio_pnl_value_display_mode();

        let (chart_points, total_value, total_text, empty_chart_text) =
            if value_mode == PnlValueDisplayMode::Percent {
                let performance_points = self.selected_portfolio_performance_points();
                let total_performance = portfolio_total_performance(&performance_points);
                (
                    performance_points,
                    total_performance,
                    total_performance
                        .map(format_signed_percent_value)
                        .unwrap_or_else(|| "—".to_string()),
                    "No portfolio performance available",
                )
            } else {
                let pnl_points = self.selected_portfolio_points();
                let total_pnl = portfolio_total_pnl(&pnl_points);
                let denomination = self.display_denomination_context();
                (
                    pnl_points,
                    total_pnl,
                    total_pnl
                        .map(|value| denomination.format_signed_value(value, 2))
                        .unwrap_or_else(|| "—".to_string()),
                    "No portfolio history available",
                )
            };

        let performance =
            portfolio_total_performance(&self.selected_portfolio_performance_points());

        let scope_label = match self.portfolio.scope {
            PortfolioScope::All => "All",
            PortfolioScope::Perp => "Perp",
        };
        let metric_label = if value_mode == PnlValueDisplayMode::Percent {
            "Total Performance"
        } else {
            "Total PnL"
        };
        let hero_label = format!("{metric_label} · {scope_label}").to_uppercase();

        let hero = view_portfolio_hero(
            &theme,
            hero_label,
            total_text,
            tokens::pnl_color(&theme, total_value),
            performance,
            value_mode == PnlValueDisplayMode::Usd,
        );

        let chart_body: Element<'_, Message> = if chart_points.len() >= 2 {
            canvas(PortfolioPnlChart {
                points: chart_points,
                value_mode,
                denomination: self.display_denomination_context(),
            })
            .width(Fill)
            .height(CHART_HEIGHT)
            .into()
        } else if self.portfolio.loading {
            container(self.loading_overlay("Loading portfolio..."))
                .width(Fill)
                .height(CHART_HEIGHT)
                .into()
        } else {
            container(
                text(empty_chart_text)
                    .size(11)
                    .font(tokens::mono())
                    .color(tokens::dim(&theme)),
            )
            .width(Fill)
            .height(CHART_HEIGHT)
            .center(Fill)
            .into()
        };

        let mut content = column![
            self.view_portfolio_control_row(),
            hero,
            self.view_portfolio_stat_strip(&theme),
            chart_body,
            self.view_portfolio_timeframe_track(),
            self.view_portfolio_daily_section(&theme),
        ]
        .spacing(BLOCK_GAP)
        .width(Fill);

        let skipped_invalid_points = self
            .portfolio_window_bucket()
            .map(|bucket| bucket.skipped_invalid_points)
            .unwrap_or_default();
        if skipped_invalid_points > 0 {
            content = content.push(
                text(format!(
                    "Invalid portfolio history points skipped: {skipped_invalid_points}"
                ))
                .size(10)
                .font(tokens::mono())
                .color(tokens::down(&theme)),
            );
        }

        if let Some(err) = &self.portfolio.last_error {
            content = content.push(
                text(format!("Stale data: {err}"))
                    .size(10)
                    .font(tokens::mono())
                    .color(tokens::accent(&theme)),
            );
        }

        let scroll = scrollable(container(content).width(Fill).padding(iced::Padding {
            top: 0.0,
            right: 12.0,
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
            .padding(BODY_PADDING)
            .into()
    }

    fn portfolio_pnl_value_display_mode(&self) -> PnlValueDisplayMode {
        self.portfolio.pnl_value_display_mode
    }
}

#[cfg(test)]
mod tests;
