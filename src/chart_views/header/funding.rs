use super::metrics::funding_column;
use crate::account::AssetContext;
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartInstance;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::{parse_finite_number, parse_positive_finite_number};
use crate::message::Message;
use iced::widget::{Column, Space, column, container, row, rule, text, tooltip};
use iced::{Element, Fill, Theme};

const FUNDING_HORIZONS: [(&str, f64); 4] =
    [("1d", 24.0), ("7d", 168.0), ("30d", 720.0), ("1y", 8_760.0)];

impl TradingTerminal {
    pub(super) fn view_chart_funding(
        &self,
        instance: &ChartInstance,
        theme: &Theme,
        now_ms: u64,
    ) -> Element<'static, Message> {
        let metric = funding_column(theme, instance.asset_ctx.as_ref(), now_ms);
        let Some(amounts) = self.chart_funding_projection(instance) else {
            return metric.into();
        };

        let content = view_funding_projection(
            amounts,
            &instance.chart.display_denomination,
            self.hide_pnl,
            theme,
        );
        tooltip(metric, content, tooltip::Position::Bottom)
            .gap(4)
            .padding(8)
            .style(container::rounded_box)
            .into()
    }

    fn chart_funding_projection(&self, instance: &ChartInstance) -> Option<[f64; 4]> {
        if !self.is_perp_coin(&instance.symbol) || self.symbol_key_is_hidden(&instance.symbol) {
            return None;
        }
        let (_, data) = self.connected_order_account_snapshot()?;
        let position = data
            .clearinghouse
            .asset_positions
            .iter()
            .find(|ap| ap.position.coin == instance.symbol)?;
        funding_projection(&position.position.szi, instance.asset_ctx.as_ref()?)
    }
}

fn view_funding_projection(
    amounts: [f64; 4],
    denomination: &DisplayDenominationContext,
    hide_pnl: bool,
    theme: &Theme,
) -> Column<'static, Message> {
    let (title, color) = if amounts[0] > 0.0 {
        ("Estimated funding · Earn", theme.palette().success)
    } else if amounts[0] < 0.0 {
        ("Estimated funding · Pay", theme.palette().danger)
    } else {
        ("Estimated funding", theme.palette().text)
    };
    let muted = theme.extended_palette().background.weak.text;
    let mut content = column![text(title).size(12), rule::horizontal(1)].spacing(4);
    for ((label, _), amount) in FUNDING_HORIZONS.into_iter().zip(amounts) {
        let value = if hide_pnl {
            denomination.hidden_mask()
        } else {
            denomination.format_signed_value(amount, 4)
        };
        content = content.push(
            row![
                text(label).size(12).color(muted),
                Space::new().width(Fill),
                text(value)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .color(color),
            ]
            .spacing(16),
        );
    }
    content
        .push(
            text("Constant rate, size & oracle price")
                .size(10)
                .color(muted),
        )
        .width(210)
}

fn funding_projection(signed_size: &str, ctx: &AssetContext) -> Option<[f64; 4]> {
    let size = parse_finite_number(signed_size)?;
    if size == 0.0 {
        return None;
    }
    let rate = parse_finite_number(ctx.funding.as_deref()?)?;
    let oracle_price = parse_positive_finite_number(ctx.oracle_px.as_deref()?)?;
    // Hyperliquid settles hourly using oracle notional. Positive rates charge
    // longs and credit shorts; these estimates express the wallet's cash flow.
    let hourly = -size * oracle_price * rate;
    let amounts = FUNDING_HORIZONS.map(|(_, hours)| hourly * hours);
    amounts
        .iter()
        .all(|amount| amount.is_finite())
        .then_some(amounts)
}

#[cfg(test)]
mod tests;
