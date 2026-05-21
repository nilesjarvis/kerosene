use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::{format_decimal_with_commas, format_size};
use crate::hype_etf_state::{HypeEtfDailyFlow, HypeEtfFund, HypeEtfTotals, HypeEtfView};
use crate::message::Message;

use iced::widget::{
    Column, Space, button, canvas, column, container, responsive, row, rule, scrollable, stack,
    text, tooltip,
};
use iced::{Color, Element, Fill, Length, Point, Rectangle, Renderer, Theme, color};

// ---------------------------------------------------------------------------
// HYPE ETF View
// ---------------------------------------------------------------------------

const FLOW_BAR_SPACING: u32 = 3;
const FLOW_CHART_HEIGHT: f32 = 126.0;
const FLOW_AXIS_HEIGHT: f32 = 1.0;
const FLOW_CHART_VERTICAL_PADDING: f32 = 8.0;

impl TradingTerminal {
    pub(crate) fn view_hype_etfs(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_hype_etfs_sized(size.width)).into()
    }

    fn view_hype_etfs_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let header = row![
            text("HYPE ETFs")
                .size(13)
                .color(theme.palette().text)
                .width(Fill),
            button(text("Refresh").size(11).center())
                .padding([3, 8])
                .on_press(Message::RefreshHypeEtfs)
                .style(button::text),
        ]
        .align_y(iced::Alignment::Center);

        let mut content = column![header, self.view_hype_etf_tabs(), rule::horizontal(1)]
            .spacing(8)
            .width(Fill);

        let mut body = Column::new().spacing(8);
        if self.hype_etfs.loading {
            body = body.push(
                row![
                    self.view_spinner(18),
                    text("Loading ETF data")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            );
        }

        if let Some(error) = &self.hype_etfs.error {
            body = body.push(
                text(error.clone())
                    .size(11)
                    .color(color!(0xff5555))
                    .width(Fill),
            );
        }

        if let Some(data) = &self.hype_etfs.data {
            for warning in &data.warnings {
                body = body.push(
                    text(warning.clone())
                        .size(11)
                        .color(color!(0xffb86c))
                        .width(Fill),
                );
            }

            let selected_funds = data.selected_funds(self.hype_etfs.view);
            if selected_funds.is_empty() {
                body = body.push(
                    text("No data returned for this ETF")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                );
            } else {
                body = body.push(summary_section(
                    &theme,
                    self.hype_etfs.view,
                    data.totals_for(self.hype_etfs.view),
                    selected_funds.len(),
                    available_width,
                    &denomination,
                ));

                let daily_flows = data.daily_flows_for(self.hype_etfs.view);
                let missing_flow_labels = selected_funds
                    .iter()
                    .filter(|fund| fund.daily_flows.is_empty())
                    .map(|fund| fund.ticker.label())
                    .collect::<Vec<_>>();
                body = body.push(daily_inflow_chart(
                    &theme,
                    self.hype_etfs.view,
                    &daily_flows,
                    &missing_flow_labels,
                    available_width,
                    &denomination,
                ));

                for fund in selected_funds {
                    body = body.push(fund_section(&theme, fund, available_width, &denomination));
                }
            }
        } else if !self.hype_etfs.loading {
            body = body.push(
                text("No ETF data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        content = content.push(scrollable(body).height(Fill));

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn view_hype_etf_tabs(&self) -> Element<'static, Message> {
        HypeEtfView::ALL
            .iter()
            .copied()
            .fold(row![].spacing(4), |tabs, view| {
                tabs.push(hype_etf_tab(view, self.hype_etfs.view == view))
            })
            .into()
    }
}

fn hype_etf_tab(view: HypeEtfView, active: bool) -> Element<'static, Message> {
    button(text(view.label()).size(11).center().width(Fill))
        .on_press(Message::HypeEtfsViewChanged(view))
        .padding([4, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = match (active, status) {
                (true, _) => theme.extended_palette().background.strong.color,
                (false, button::Status::Hovered) => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}

fn summary_section(
    theme: &Theme,
    view: HypeEtfView,
    totals: HypeEtfTotals,
    fund_count: usize,
    available_width: f32,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let title = if view == HypeEtfView::All {
        format!("All HYPE ETFs ({fund_count})")
    } else {
        view.label().to_string()
    };

    column![
        text(title).size(12).color(theme.palette().text),
        metric_grid(
            vec![
                metric(
                    "Total Assets",
                    format_usd_value(totals.net_assets_usd, 2, denomination),
                    None
                ),
                metric("HYPE Exposure", format_hype(totals.hype_exposure), None),
                metric("Share Volume", format_amount(totals.daily_volume), None),
                metric(
                    "Premium/Discount",
                    format_pct(totals.weighted_premium_discount_pct),
                    totals
                        .weighted_premium_discount_pct
                        .map(|value| signed_color(theme, value)),
                ),
                metric(
                    "30D Spread",
                    format_pct(totals.weighted_median_spread_pct),
                    None,
                ),
                metric(
                    "Fund Shares",
                    format_amount(totals.shares_outstanding),
                    None,
                ),
            ],
            available_width,
        )
    ]
    .spacing(6)
    .into()
}

fn daily_inflow_chart(
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
            .color(signed_color(theme, net_flow)),
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

fn flow_chart(
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
            signed_color(theme, flow.amount_usd)
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
            text(tooltip_text).size(10).font(crate::app_fonts::monospace_font()),
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

#[derive(Debug, Clone)]
struct CumulativeInflowLine {
    values: Vec<f64>,
    scale: FlowChartScale,
}

impl canvas::Program<Message> for CumulativeInflowLine {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        let points = cumulative_line_points(&self.values, bounds.width, bounds.height, self.scale);
        if points.is_empty() {
            return vec![frame.into_geometry()];
        }

        let line_color = theme.palette().primary;
        if points.len() == 1 {
            frame.fill(&canvas::Path::circle(points[0], 2.8), line_color);
            return vec![frame.into_geometry()];
        }

        let line = canvas::Path::new(|path| {
            for (idx, point) in points.iter().copied().enumerate() {
                if idx == 0 {
                    path.move_to(point);
                } else {
                    path.line_to(point);
                }
            }
        });
        frame.stroke(
            &line,
            canvas::Stroke::default()
                .with_color(line_color)
                .with_width(2.0)
                .with_line_cap(canvas::LineCap::Round)
                .with_line_join(canvas::LineJoin::Round),
        );

        for point in points {
            frame.fill(&canvas::Path::circle(point, 2.2), line_color);
            frame.stroke(
                &canvas::Path::circle(point, 2.2),
                canvas::Stroke::default()
                    .with_color(theme.extended_palette().background.weak.color)
                    .with_width(1.0),
            );
        }

        vec![frame.into_geometry()]
    }
}

fn cumulative_inflows(flows: &[HypeEtfDailyFlow]) -> Vec<f64> {
    let mut total = 0.0;
    flows
        .iter()
        .map(|flow| {
            total += flow.amount_usd;
            total
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct FlowChartScale {
    zero_y: f32,
    positive_height: f32,
    negative_height: f32,
    max_positive: f64,
    max_negative: f64,
    top_padding: f32,
    bottom_padding: f32,
}

fn flow_chart_scale(values: &[f64], height: f32) -> FlowChartScale {
    let max_positive = values
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .fold(0.0_f64, f64::max);
    let max_negative = values
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value < 0.0)
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
    let top_padding = FLOW_CHART_VERTICAL_PADDING.min(height * 0.4);
    let bottom_padding = FLOW_CHART_VERTICAL_PADDING.min(height * 0.4);
    let usable_height = (height - top_padding - bottom_padding - FLOW_AXIS_HEIGHT).max(1.0);

    let (positive_height, negative_height) = match (max_positive > 0.0, max_negative > 0.0) {
        (true, true) => {
            let positive_share = max_positive / (max_positive + max_negative);
            let positive_height = (usable_height * positive_share as f32).clamp(1.0, usable_height);
            (positive_height, usable_height - positive_height)
        }
        (true, false) => (usable_height, 0.0),
        (false, true) => (0.0, usable_height),
        (false, false) => (usable_height * 0.5, usable_height * 0.5),
    };

    FlowChartScale {
        zero_y: top_padding + positive_height,
        positive_height,
        negative_height,
        max_positive: max_positive.max(1.0),
        max_negative: max_negative.max(1.0),
        top_padding,
        bottom_padding,
    }
}

fn flow_bar_layout(value: f64, scale: FlowChartScale) -> (f32, f32, f32, f32) {
    let value = if value.is_finite() { value } else { 0.0 };
    let min_visible_height = 2.0_f32.min(scale.positive_height.max(scale.negative_height));
    let positive_height = if value > 0.0 {
        ((value / scale.max_positive) as f32 * scale.positive_height)
            .max(min_visible_height)
            .min(scale.positive_height)
    } else {
        0.0
    };
    let negative_height = if value < 0.0 {
        ((value.abs() / scale.max_negative) as f32 * scale.negative_height)
            .max(min_visible_height)
            .min(scale.negative_height)
    } else {
        0.0
    };
    let axis_bottom = scale.zero_y + FLOW_AXIS_HEIGHT;
    let top_spacer = scale.top_padding + (scale.positive_height - positive_height).max(0.0);
    let bottom_spacer = scale.bottom_padding + (scale.negative_height - negative_height).max(0.0);

    debug_assert!((top_spacer + positive_height - scale.zero_y).abs() < 0.5);
    debug_assert!((axis_bottom + negative_height + bottom_spacer - FLOW_CHART_HEIGHT).abs() < 0.5);

    (top_spacer, positive_height, negative_height, bottom_spacer)
}

fn cumulative_line_points(
    values: &[f64],
    width: f32,
    height: f32,
    scale: FlowChartScale,
) -> Vec<Point> {
    if values.is_empty() || width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }

    let line_scale = flow_chart_scale(values, height);
    let count = values.len();
    let spacing = FLOW_BAR_SPACING as f32;
    let available_width = (width - spacing * count.saturating_sub(1) as f32).max(count as f32);
    let bar_width = available_width / count as f32;

    values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .map(|(idx, value)| {
            let x = bar_width * 0.5 + idx as f32 * (bar_width + spacing);
            let y = if value >= 0.0 {
                scale.zero_y - (value / line_scale.max_positive) as f32 * scale.positive_height
            } else {
                scale.zero_y
                    + FLOW_AXIS_HEIGHT
                    + (value.abs() / line_scale.max_negative) as f32 * scale.negative_height
            };
            Point::new(x.clamp(0.0, width), y.clamp(0.0, height))
        })
        .collect()
}

fn fund_section(
    theme: &Theme,
    fund: &HypeEtfFund,
    available_width: f32,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let title = row![
        text(fund.ticker.label())
            .size(12)
            .color(theme.palette().primary),
        text(fund.ticker.name())
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(Fill),
        text(
            fund.as_of_date
                .clone()
                .unwrap_or_else(|| "date n/a".to_string()),
        )
        .size(10)
        .color(theme.extended_palette().background.weak.text),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let mut items = vec![
        metric(
            "Assets",
            format_usd_value(fund.net_assets_usd, 2, denomination),
            None,
        ),
        metric("HYPE", format_hype(fund.hype_exposure), None),
        metric(
            "NAV",
            format_usd_value(fund.nav_per_share, 2, denomination),
            None,
        ),
        metric(
            "Market",
            format_usd_value(fund.market_price, 2, denomination),
            None,
        ),
        metric(
            "NAV 1D",
            format_pct(fund.nav_change_pct),
            fund.nav_change_pct.map(|value| signed_color(theme, value)),
        ),
        metric(
            "Market 1D",
            format_pct(fund.market_price_change_pct),
            fund.market_price_change_pct
                .map(|value| signed_color(theme, value)),
        ),
        metric(
            "Premium/Discount",
            format_pct(fund.premium_discount_pct),
            fund.premium_discount_pct
                .map(|value| signed_color(theme, value)),
        ),
        metric("Volume", format_amount(fund.daily_volume), None),
        metric("30D Spread", format_pct(fund.median_spread_pct), None),
        metric(
            "HYPE Px",
            format_usd_value(fund.hype_reference_price, 2, denomination),
            None,
        ),
    ];

    if fund.thirty_day_volume.is_some() {
        items.push(metric(
            "30D Volume",
            format_amount(fund.thirty_day_volume),
            None,
        ));
    }
    if fund.staking_net_rate_pct.is_some() {
        items.push(metric(
            "Net Staking",
            format_pct(fund.staking_net_rate_pct),
            None,
        ));
    }
    if fund.staking_current_pct.is_some() {
        items.push(metric(
            "Assets Staked",
            format_pct(fund.staking_current_pct),
            None,
        ));
    }

    let mut section = column![title, metric_grid(items, available_width)].spacing(6);
    if let Some(updated_at) = &fund.updated_at {
        section = section.push(
            text(format!("Updated {updated_at}"))
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

#[derive(Debug, Clone)]
struct Metric {
    label: &'static str,
    value: String,
    color: Option<Color>,
}

fn metric(label: &'static str, value: String, color: Option<Color>) -> Metric {
    Metric {
        label,
        value,
        color,
    }
}

fn metric_grid(metrics: Vec<Metric>, available_width: f32) -> Element<'static, Message> {
    let columns = if available_width >= 560.0 {
        3
    } else if available_width >= 360.0 {
        2
    } else {
        1
    };

    let mut grid = Column::new().spacing(6);
    for chunk in metrics.chunks(columns) {
        let mut line = row![].spacing(6).width(Fill);
        for item in chunk {
            line = line.push(metric_card(item.clone()));
        }
        grid = grid.push(line);
    }
    grid.into()
}

fn metric_card(metric: Metric) -> Element<'static, Message> {
    let value_color = metric.color;
    container(
        column![
            text(metric.label)
                .size(10)
                .color(color!(0x888888))
                .width(Fill),
            text(metric.value)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .style(move |theme: &Theme| text::Style {
                    color: Some(value_color.unwrap_or(theme.palette().text)),
                })
                .width(Fill),
        ]
        .spacing(2),
    )
    .width(Fill)
    .padding([6, 8])
    .style(move |theme: &Theme| container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    })
    .into()
}

fn signed_color(theme: &Theme, value: f64) -> Color {
    if value > 0.0 {
        theme.palette().success
    } else if value < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}

fn format_usd_value(
    value: Option<f64>,
    decimals: usize,
    denomination: &DisplayDenominationContext,
) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| denomination.format_value(value, decimals))
        .unwrap_or_else(|| "n/a".to_string())
}

fn format_amount(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(format_size)
        .unwrap_or_else(|| "n/a".to_string())
}

fn format_hype(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{} HYPE", format_decimal_with_commas(value, 0)))
        .unwrap_or_else(|| "n/a".to_string())
}

fn format_pct(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:+.2}%"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn format_signed_usd_amount(value: f64, denomination: &DisplayDenominationContext) -> String {
    let value = if value.abs() < 0.005 { 0.0 } else { value };
    denomination.format_signed_value(value, 2)
}

fn short_flow_date(date: &str) -> String {
    date.get(5..).unwrap_or(date).replace('-', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn daily_flow(amount_usd: f64) -> HypeEtfDailyFlow {
        HypeEtfDailyFlow {
            date: "2026-05-20".to_string(),
            amount_usd,
        }
    }

    #[test]
    fn cumulative_inflows_tracks_running_total() {
        let flows = vec![daily_flow(100.0), daily_flow(-25.0), daily_flow(10.0)];

        assert_eq!(cumulative_inflows(&flows), vec![100.0, 75.0, 85.0]);
    }

    #[test]
    fn cumulative_line_points_stay_inside_chart_bounds() {
        let values = [100.0, 50.0, 125.0];
        let scale = flow_chart_scale(&values, FLOW_CHART_HEIGHT);
        let points = cumulative_line_points(&values, 300.0, FLOW_CHART_HEIGHT, scale);

        assert_eq!(points.len(), 3);
        assert!(points[0].x < points[1].x);
        assert!(points[1].x < points[2].x);
        assert!(
            points
                .iter()
                .all(|point| point.x >= 0.0 && point.x <= 300.0)
        );
        assert!(
            points
                .iter()
                .all(|point| point.y >= 0.0 && point.y <= FLOW_CHART_HEIGHT)
        );
    }

    #[test]
    fn cumulative_line_uses_bar_zero_baseline() {
        let values = [-100.0, 0.0, 100.0];
        let scale = flow_chart_scale(&values, FLOW_CHART_HEIGHT);
        let points = cumulative_line_points(&values, 300.0, FLOW_CHART_HEIGHT, scale);
        let zero_y = scale.zero_y;

        assert!(points[0].y > zero_y);
        assert_eq!(points[1].y, zero_y);
        assert!(points[2].y < zero_y);
    }

    #[test]
    fn positive_only_bars_use_most_of_chart_height() {
        let values = [100.0, 50.0, 25.0];
        let scale = flow_chart_scale(&values, FLOW_CHART_HEIGHT);
        let (_top_spacer, positive_height, negative_height, bottom_spacer) =
            flow_bar_layout(100.0, scale);

        assert!(positive_height > FLOW_CHART_HEIGHT * 0.75);
        assert_eq!(negative_height, 0.0);
        assert!((bottom_spacer - scale.bottom_padding).abs() < 0.1);
    }
}
