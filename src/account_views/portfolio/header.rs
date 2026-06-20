use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, column, container, row, text};
use iced::{Border, Color, Element, Fill, Theme};

use super::tokens;
use super::totals::format_signed_percent_value;

// ---------------------------------------------------------------------------
// Hero + Stat Strip
// ---------------------------------------------------------------------------

impl TradingTerminal {
    /// Equity = current account value, sourced like the account summary widget.
    pub(super) fn portfolio_equity_usdc(&self) -> Option<f64> {
        self.connected_order_account_snapshot()
            .and_then(|(_, data)| account_equity_usdc(data))
    }

    /// Available = withdrawable / free margin for the connected account.
    pub(super) fn portfolio_available_usdc(&self) -> Option<f64> {
        self.connected_order_account_snapshot()
            .and_then(|(_, data)| data.available_margin_usdc())
    }

    /// Stat strip: Equity · Available · ROI 30D between hairlines.
    pub(super) fn view_portfolio_stat_strip(&self, theme: &Theme) -> Element<'static, Message> {
        let denomination = self.display_denomination_context();
        let equity = self.portfolio_equity_usdc();
        let available = self.portfolio_available_usdc();
        let roi = self.portfolio_roi_30d_percent();

        let money = |value: Option<f64>| {
            value
                .map(|value| denomination.format_value(value, 0))
                .unwrap_or_else(|| "—".to_string())
        };
        let roi_text = roi
            .map(format_signed_percent_value)
            .unwrap_or_else(|| "—".to_string());

        let stats = row![
            stat_column(theme, "Equity", money(equity), tokens::text(theme)),
            stat_column(theme, "Available", money(available), tokens::accent(theme)),
            stat_column(theme, "ROI 30D", roi_text, tokens::pnl_color(theme, roi)),
        ]
        .spacing(10)
        .width(Fill);

        column![
            hairline(theme),
            container(stats).padding([11, 0]).width(Fill),
            hairline(theme),
        ]
        .width(Fill)
        .into()
    }
}

/// Hero block: dim label, oversized sign-colored value, and an optional
/// performance chip pinned to the bottom-right.
pub(super) fn view_portfolio_hero(
    theme: &Theme,
    label: String,
    value_text: String,
    value_color: Color,
    performance: Option<f64>,
    show_chip: bool,
) -> Element<'static, Message> {
    let headline = column![
        text(label)
            .size(10)
            .font(tokens::mono())
            .color(tokens::dim(theme)),
        text(value_text)
            .size(33)
            .font(tokens::mono_semibold())
            .color(value_color),
    ]
    .spacing(5)
    .width(Fill);

    let mut hero = row![headline].align_y(iced::Alignment::End).width(Fill);
    if show_chip && let Some(performance) = performance {
        hero = hero.push(performance_chip(theme, performance));
    }
    hero.into()
}

fn performance_chip(theme: &Theme, performance: f64) -> Element<'static, Message> {
    let gain = performance >= 0.0;
    let (color, background) = if gain {
        (tokens::up(theme), tokens::up_wash(theme))
    } else {
        (tokens::down(theme), tokens::down_wash(theme))
    };
    let arrow = if gain { '\u{25B2}' } else { '\u{25BC}' };
    let label = format!("{arrow} {:.2}%", performance.abs());

    container(
        text(label)
            .size(12)
            .font(tokens::mono_semibold())
            .color(color),
    )
    .padding([3, 9])
    .style(move |_theme: &Theme| container::Style {
        background: Some(background.into()),
        border: Border {
            color: Color { a: 0.5, ..color },
            width: 1.0,
            radius: 3.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

fn stat_column(
    theme: &Theme,
    label: &str,
    value: String,
    value_color: Color,
) -> Element<'static, Message> {
    column![
        text(label.to_uppercase())
            .size(9)
            .font(tokens::mono())
            .color(tokens::dim(theme)),
        text(value)
            .size(15)
            .font(tokens::mono_semibold())
            .color(value_color),
    ]
    .spacing(3)
    .width(Fill)
    .into()
}

fn hairline(theme: &Theme) -> Element<'static, Message> {
    let color = tokens::border(theme);
    container(Space::new().width(Fill).height(1))
        .width(Fill)
        .style(move |_theme: &Theme| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

/// Account equity in USDC: the directly-reported account value when present.
fn account_equity_usdc(data: &AccountData) -> Option<f64> {
    data.account_value_usdc()
}
