use super::super::formatting::format_signed_usd_amount;
use super::line::CumulativeInflowLine;
use super::scale::{
    FLOW_AXIS_HEIGHT, FLOW_BAR_SPACING, FLOW_CHART_HEIGHT, cumulative_inflows, flow_bar_layout,
    flow_chart_scale,
};
use crate::denomination::DisplayDenominationContext;
use crate::helpers::signed_number_color;
use crate::hype_etf_state::HypeEtfDailyFlow;
use crate::message::Message;

use iced::widget::{Space, canvas, column, container, row, stack, text, tooltip};
use iced::{Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Daily Flow Bars
// ---------------------------------------------------------------------------

pub(super) fn flow_chart(
    theme: &Theme,
    flows: &[HypeEtfDailyFlow],
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let cumulative_values = cumulative_inflows(flows);
    let flow_values = flows.iter().map(|flow| flow.amount_usd).collect::<Vec<_>>();
    let scale = flow_chart_scale(&flow_values, FLOW_CHART_HEIGHT);
    let axis_color = Color {
        a: 0.24,
        ..theme.palette().text
    };

    let mut bars = row![].spacing(FLOW_BAR_SPACING).width(Fill);
    for (flow, cumulative) in flows.iter().cloned().zip(cumulative_values.iter().copied()) {
        let (top_spacer, positive_height, negative_height, bottom_spacer) =
            flow_bar_layout(flow.amount_usd, scale);
        let bar_color = if flow.amount_usd == 0.0 {
            axis_color
        } else {
            signed_number_color(flow.amount_usd, theme)
        };
        let tooltip_text = format!(
            "{}\nDaily {}\nCumulative {}",
            flow.date,
            format_signed_usd_amount(flow.amount_usd, denomination),
            format_signed_usd_amount(cumulative, denomination),
        );

        let bar = column![
            container(Space::new()).height(Length::Fixed(top_spacer)),
            flow_bar_segment(positive_height, bar_color),
            flow_bar_segment(FLOW_AXIS_HEIGHT, axis_color),
            flow_bar_segment(negative_height, bar_color),
            container(Space::new()).height(Length::Fixed(bottom_spacer)),
        ]
        .width(Fill)
        .height(Length::Fixed(FLOW_CHART_HEIGHT));

        let wrapped_bar = tooltip(
            bar,
            text(tooltip_text)
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            iced::widget::tooltip::Position::Top,
        )
        .gap(4)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            text_color: Some(theme.palette().text),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        bars = bars.push(wrapped_bar);
    }

    let bar_layer: Element<'static, Message> = container(bars)
        .width(Fill)
        .height(Length::Fixed(FLOW_CHART_HEIGHT))
        .into();
    let line_layer: Element<'static, Message> = canvas(CumulativeInflowLine {
        values: cumulative_values,
        scale,
    })
    .width(Fill)
    .height(Length::Fixed(FLOW_CHART_HEIGHT))
    .into();

    stack(vec![bar_layer, line_layer])
        .width(Fill)
        .height(Length::Fixed(FLOW_CHART_HEIGHT))
        .into()
}

fn flow_bar_segment(height: f32, color: Color) -> Element<'static, Message> {
    container(Space::new())
        .width(Fill)
        .height(Length::Fixed(height))
        .style(move |_| container::Style {
            background: Some(color.into()),
            ..Default::default()
        })
        .into()
}
