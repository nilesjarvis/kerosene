mod components;

use crate::app_state::TradingTerminal;
use crate::chart::CandlestickChart;
use crate::chart_state::ChartId;
use crate::helpers::format_price;
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, stack, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Chart Quick Order Overlay
// ---------------------------------------------------------------------------

impl TradingTerminal {
    /// Render the quick order floating card layered on top of the chart canvas.
    pub(crate) fn view_quick_order_card<'a>(
        &'a self,
        chart_id: ChartId,
        form: &'a QuickOrderForm,
        chart_canvas: iced::widget::Canvas<&'a CandlestickChart, Message>,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let price_str = format_price(form.price);
        let type_label = if form.is_limit {
            format!("Limit @ ${price_str}")
        } else {
            "Market Order".to_string()
        };

        let title_row = Self::quick_order_title_row(chart_id, form, type_label);
        let qty_input = Self::quick_order_quantity_input(chart_id, form);
        let action_row = self.quick_order_action_row(chart_id);
        let fee_el = self.quick_order_fee_estimate(form);
        let presets_scroll = self.quick_order_presets_scroll(chart_id, form);

        let qty_header = row![
            text("Qty")
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(Fill),
            presets_scroll
        ]
        .align_y(iced::Alignment::Center);

        let card_content =
            column![title_row, qty_header, qty_input, fee_el, action_row,].spacing(6);

        let card_width = 220.0;
        let card_height = 170.0;
        let max_x = (form.chart_w - card_width).max(0.0);
        let max_y = (form.chart_h - card_height).max(0.0);
        let pad_left = form.click_x.clamp(0.0, max_x);
        let pad_top = form.click_y.clamp(0.0, max_y);

        let card = container(card_content)
            .width(card_width)
            .padding(8)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                border: iced::Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.strong.color,
                },
                ..Default::default()
            });

        let positioned_card: Element<'_, Message> = container(card)
            .width(Fill)
            .height(Fill)
            .padding(iced::Padding {
                top: pad_top,
                right: 0.0,
                bottom: 0.0,
                left: pad_left,
            })
            .into();

        let canvas_el: Element<'_, Message> = chart_canvas.width(Fill).height(Fill).into();

        let dismiss_backdrop: Element<'_, Message> = button(text(""))
            .on_press(Message::CloseQuickOrder(chart_id))
            .width(Fill)
            .height(Fill)
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            })
            .into();

        stack![canvas_el, dismiss_backdrop, positioned_card]
            .width(Fill)
            .height(Fill)
            .into()
    }
}
