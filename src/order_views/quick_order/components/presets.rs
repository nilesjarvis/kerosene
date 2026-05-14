use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use iced::widget::{button, row, scrollable, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::order_views::quick_order) fn quick_order_presets_scroll<'a>(
        &'a self,
        chart_id: ChartId,
        form: &QuickOrderForm,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let presets = if self.preset_is_usd {
            if form.is_limit {
                &self.order_presets.limit_usd
            } else {
                &self.order_presets.market_usd
            }
        } else if form.is_limit {
            &self.order_presets.limit_coin
        } else {
            &self.order_presets.market_coin
        };
        let symbol = self
            .charts
            .get(&chart_id)
            .map(|instance| instance.symbol.as_str())
            .unwrap_or(self.active_symbol.as_str());
        let decimals = self
            .exchange_symbols
            .iter()
            .find(|exchange_symbol| exchange_symbol.key == symbol)
            .map(|exchange_symbol| exchange_symbol.sz_decimals as usize)
            .unwrap_or(4);
        let reference_price = if form.is_limit {
            (form.price.is_finite() && form.price > 0.0).then_some(form.price)
        } else {
            self.resolve_mid_for_symbol(symbol)
                .filter(|price| price.is_finite() && *price > 0.0)
        };

        let mut preset_row = row![].spacing(4);
        for p in presets {
            let preset_qty_str = quick_order_preset_quantity(
                p.size,
                self.preset_is_usd,
                form.quantity_is_usd,
                reference_price,
                decimals,
            );

            if !preset_qty_str.is_empty() {
                let btn = button(text(&p.label).size(9).color(theme.palette().text))
                    .on_press(Message::QuickOrderQtyChanged(chart_id, preset_qty_str))
                    .padding([2, 6])
                    .style(|theme: &Theme, status| {
                        let bg = match status {
                            button::Status::Hovered => {
                                theme.extended_palette().background.strong.color
                            }
                            _ => theme.extended_palette().background.weak.color,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            text_color: theme.palette().text,
                            border: iced::Border {
                                radius: 2.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    });
                preset_row = preset_row.push(btn);
            }
        }

        scrollable(preset_row)
            .direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::new()
                    .width(0)
                    .margin(0)
                    .scroller_width(0),
            ))
            .width(Fill)
            .into()
    }
}

fn quick_order_preset_quantity(
    size: f64,
    preset_is_usd: bool,
    quantity_is_usd: bool,
    reference_price: Option<f64>,
    decimals: usize,
) -> String {
    if !size.is_finite() || size <= 0.0 {
        return String::new();
    }

    match (preset_is_usd, quantity_is_usd) {
        (true, true) => format!("{size:.2}"),
        (true, false) => reference_price
            .filter(|price| price.is_finite() && *price > 0.0)
            .map(|price| format!("{:.decimals$}", size / price))
            .unwrap_or_default(),
        (false, true) => reference_price
            .filter(|price| price.is_finite() && *price > 0.0)
            .map(|price| format!("{:.2}", size * price))
            .unwrap_or_default(),
        (false, false) => format!("{size:.decimals$}"),
    }
}

#[cfg(test)]
mod tests {
    use super::quick_order_preset_quantity;

    #[test]
    fn quick_order_presets_render_in_selected_denomination() {
        assert_eq!(
            quick_order_preset_quantity(250.0, true, true, Some(100.0), 4),
            "250.00"
        );
        assert_eq!(
            quick_order_preset_quantity(250.0, true, false, Some(100.0), 4),
            "2.5000"
        );
        assert_eq!(
            quick_order_preset_quantity(2.5, false, true, Some(100.0), 4),
            "250.00"
        );
        assert_eq!(
            quick_order_preset_quantity(2.5, false, false, Some(100.0), 4),
            "2.5000"
        );
    }

    #[test]
    fn quick_order_presets_require_price_for_cross_denomination() {
        assert_eq!(quick_order_preset_quantity(250.0, true, false, None, 4), "");
        assert_eq!(quick_order_preset_quantity(2.5, false, true, None, 4), "");
    }
}
