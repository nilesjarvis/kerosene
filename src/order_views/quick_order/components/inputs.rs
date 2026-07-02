use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::helpers::{self, parse_number};
use crate::message::Message;
use crate::order_execution::{QuickOrderForm, order_size_from_quantity_input};
use iced::widget::{Space, button, column, row, slider, text, text_input};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::order_views::quick_order) fn quick_order_quantity_input<'a>(
        chart_id: ChartId,
        form: &'a QuickOrderForm,
    ) -> Element<'a, Message> {
        let id = chart_id;
        text_input("Quantity", &form.quantity)
            .id(iced::widget::Id::from(format!(
                "quick_order_qty_{}",
                chart_id
            )))
            .style(helpers::text_input_style)
            .on_input(move |q| Message::QuickOrderQtyChanged(id, q.into()))
            .size(12)
            .padding([4, 6])
            .into()
    }

    pub(in crate::order_views::quick_order) fn quick_order_denomination_button<'a>(
        chart_id: ChartId,
        quantity_is_usd: bool,
    ) -> button::Button<'a, Message> {
        button(
            text(if quantity_is_usd { "$" } else { "COIN" })
                .size(10)
                .center(),
        )
        .on_press(Message::QuickOrderToggleDenomination(chart_id))
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
    }

    pub(in crate::order_views::quick_order) fn quick_order_size_controls<'a>(
        &'a self,
        chart_id: ChartId,
        form: &QuickOrderForm,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let percent_slider = slider(0.0..=100.0, form.percentage, move |value| {
            Message::QuickOrderPercentageChanged(chart_id, value)
        })
        .step(1.0)
        .style(|theme: &Theme, status| {
            let palette = theme.palette();
            let mut style = slider::default(theme, status);
            style.handle.background = palette.primary.into();
            style.handle.border_color = palette.primary;
            style.rail.backgrounds.0 = palette.primary.into();
            style.rail.backgrounds.1 = Color {
                a: 0.2,
                ..palette.text
            }
            .into();
            style
        });

        let slider_label = text(format!("{:.0}%", form.percentage))
            .size(10)
            .color(theme.extended_palette().background.weak.text);
        let slider_row = row![percent_slider, Space::new().width(6.0), slider_label]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        let quick_buttons = [25.0, 50.0, 75.0, 100.0]
            .into_iter()
            .fold(row![].spacing(4), |row, pct| {
                row.push(quick_order_percent_button(chart_id, pct, form.percentage))
            });

        column![slider_row, quick_buttons].spacing(4).into()
    }

    pub(in crate::order_views::quick_order) fn quick_order_fee_estimate<'a>(
        &'a self,
        chart_id: ChartId,
        form: &QuickOrderForm,
    ) -> Element<'a, Message> {
        let theme = self.theme();

        match self.quick_order_fee_inputs(chart_id, form) {
            Some((px, qty, is_spot)) => {
                let m_text = if let Some((fee_amt, _)) = self.estimate_fee(px, qty, true, is_spot) {
                    format!("Maker: ${fee_amt:.2}")
                } else {
                    "Maker: \u{2014}".to_string()
                };

                let t_text = if let Some((fee_amt, _)) = self.estimate_fee(px, qty, false, is_spot)
                {
                    format!("Taker: ${fee_amt:.2}")
                } else {
                    "Taker: \u{2014}".to_string()
                };

                text(format!("Est. Fees: {m_text} | {t_text}"))
                    .size(9)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            }
            None => text("Est. Fees: Maker: \u{2014} | Taker: \u{2014}")
                .size(9)
                .color(theme.extended_palette().background.weak.text)
                .into(),
        }
    }

    /// Fee-estimate inputs (price, size, is_spot) for a quick-order HUD. The
    /// HUD is a per-chart overlay, so every market-dependent input must come
    /// from that chart's symbol — not the globally active symbol, which can
    /// point at a different market (e.g. for detached chart windows).
    fn quick_order_fee_inputs(
        &self,
        chart_id: ChartId,
        form: &QuickOrderForm,
    ) -> Option<(f64, f64, bool)> {
        let symbol = self
            .charts
            .get(&chart_id)
            .map(|instance| instance.symbol.as_str())?;
        let is_spot = self.is_spot_coin(symbol);
        let fee_price = if form.is_limit {
            Some(form.price)
        } else {
            self.resolve_mid_for_symbol(symbol)
        }?;
        let sz_decimals = self
            .exchange_symbols
            .iter()
            .find(|exchange_symbol| exchange_symbol.key == symbol)
            .map(|exchange_symbol| exchange_symbol.sz_decimals)?;
        let fee_qty = quick_order_fee_quantity(form, fee_price, sz_decimals)?;
        Some((fee_price, fee_qty, is_spot))
    }
}

fn quick_order_percent_button<'a>(
    chart_id: ChartId,
    pct: f32,
    current_pct: f32,
) -> button::Button<'a, Message> {
    let selected = (current_pct - pct).abs() < 0.5;
    button(text(format!("{pct:.0}%")).size(9).center())
        .on_press(Message::QuickOrderPercentageChanged(chart_id, pct))
        .padding([2, 6])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = if selected {
                theme.palette().primary
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
}

fn quick_order_fee_quantity(form: &QuickOrderForm, price: f64, sz_decimals: u32) -> Option<f64> {
    let quantity = parse_number(&form.quantity)?;
    order_size_from_quantity_input(quantity, price, form.quantity_is_usd, sz_decimals)
}

#[cfg(test)]
mod tests;
