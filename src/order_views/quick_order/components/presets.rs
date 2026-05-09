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

        let mut preset_row = row![].spacing(4);
        for p in presets {
            let preset_qty_str = if self.preset_is_usd {
                let px = if form.is_limit {
                    (form.price.is_finite() && form.price > 0.0).then_some(form.price)
                } else {
                    self.resolve_mid_for_symbol(&self.active_symbol)
                        .filter(|price| price.is_finite() && *price > 0.0)
                };
                if let Some(px) = px {
                    format!("{:.6}", p.size / px)
                } else {
                    String::new()
                }
            } else {
                format!("{:.6}", p.size)
            };

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
