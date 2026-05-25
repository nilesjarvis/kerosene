use super::formatting::{format_signed_usd_amount, short_flow_date};
use crate::denomination::DisplayDenominationContext;
use crate::helpers::signed_number_color;
use crate::hype_etf_state::{HypeEtfDailyFlow, HypeEtfView};
use crate::message::Message;

use iced::widget::{column, container, row, text};
use iced::{Color, Element, Fill, Theme};

mod bars;
mod line;
mod scale;

use bars::flow_chart;
#[cfg(test)]
pub(super) use scale::cumulative_line_points;
#[cfg(test)]
pub(super) use scale::{FLOW_CHART_HEIGHT, cumulative_inflows, flow_bar_layout, flow_chart_scale};

// ---------------------------------------------------------------------------
// Daily Flow Chart
// ---------------------------------------------------------------------------

pub(super) fn daily_inflow_chart(
    theme: &Theme,
    view: HypeEtfView,
    flows: &[HypeEtfDailyFlow],
    missing_flow_labels: &[&'static str],
    available_width: f32,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let max_bars = if available_width >= 560.0 {
        30
    } else if available_width >= 360.0 {
        18
    } else {
        10
    };
    let bars = latest_daily_flows(flows, max_bars);
    let net_flow = bars.iter().map(|flow| flow.amount_usd).sum::<f64>();

    let header = row![
        text(daily_inflow_title(view))
            .size(11)
            .color(theme.palette().text)
            .width(Fill),
        text(format_signed_usd_amount(net_flow, denomination))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(signed_number_color(net_flow, theme)),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let mut section = column![header].spacing(6);
    if bars.is_empty() {
        section = section.push(
            text("No daily inflow history")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        );
    } else {
        section = section.push(flow_chart(theme, &bars, denomination));
        if let Some((first, last)) = bars.first().zip(bars.last()) {
            section = section.push(
                row![
                    text(short_flow_date(&first.date))
                        .size(10)
                        .font(crate::app_fonts::monospace_font())
                        .color(theme.extended_palette().background.weak.text)
                        .width(Fill),
                    text(short_flow_date(&last.date))
                        .size(10)
                        .font(crate::app_fonts::monospace_font())
                        .color(theme.extended_palette().background.weak.text),
                ]
                .width(Fill)
                .spacing(8),
            );
        }
    }

    if !missing_flow_labels.is_empty() {
        section = section.push(
            text(format!(
                "Missing flow history: {}",
                missing_flow_labels.join(", ")
            ))
            .size(10)
            .color(theme.extended_palette().background.weak.text),
        );
    }

    container(section)
        .width(Fill)
        .padding([8, 10])
        .style(move |theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.22,
                    ..theme.extended_palette().background.strong.color
                },
            },
            ..Default::default()
        })
        .into()
}

fn daily_inflow_title(view: HypeEtfView) -> String {
    if view == HypeEtfView::All {
        "Combined Daily Inflow".to_string()
    } else {
        format!("{} Daily Inflow", view.label())
    }
}

fn latest_daily_flows(flows: &[HypeEtfDailyFlow], max_bars: usize) -> Vec<HypeEtfDailyFlow> {
    flows
        .iter()
        .rev()
        .take(max_bars)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
