use crate::app_state::TradingTerminal;
use crate::helpers::{format_decimal_with_commas, format_size, format_usd};
use crate::hype_etf_state::{HypeEtfDailyFlow, HypeEtfFund, HypeEtfTotals, HypeEtfView};
use crate::message::Message;

use iced::widget::{
    Column, Space, button, column, container, responsive, row, rule, scrollable, text, tooltip,
};
use iced::{Color, Element, Fill, Length, Theme, color};

// ---------------------------------------------------------------------------
// HYPE ETF View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_hype_etfs(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_hype_etfs_sized(size.width)).into()
    }

    fn view_hype_etfs_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
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
                ));

                for fund in selected_funds {
                    body = body.push(fund_section(&theme, fund, available_width));
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
                    format_usd_value(totals.net_assets_usd, 2),
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
        text(format_signed_usd_amount(net_flow))
            .size(11)
            .font(iced::Font::MONOSPACE)
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
        section = section.push(flow_bars(theme, &bars));
        if let Some((first, last)) = bars.first().zip(bars.last()) {
            section = section.push(
                row![
                    text(short_flow_date(&first.date))
                        .size(10)
                        .font(iced::Font::MONOSPACE)
                        .color(theme.extended_palette().background.weak.text)
                        .width(Fill),
                    text(short_flow_date(&last.date))
                        .size(10)
                        .font(iced::Font::MONOSPACE)
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

fn flow_bars(theme: &Theme, flows: &[HypeEtfDailyFlow]) -> Element<'static, Message> {
    let max_abs = flows
        .iter()
        .map(|flow| flow.amount_usd.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let max_bar_height = 42.0;
    let axis_color = Color {
        a: 0.24,
        ..theme.palette().text
    };

    let mut chart = row![].spacing(3).width(Fill);
    for flow in flows.iter().cloned() {
        let scaled = ((flow.amount_usd.abs() / max_abs) as f32 * max_bar_height).max(0.0);
        let fill_height = if flow.amount_usd.abs() > 0.0 {
            scaled.max(2.0)
        } else {
            1.0
        };
        let positive_height = if flow.amount_usd >= 0.0 {
            fill_height
        } else {
            0.0
        };
        let negative_height = if flow.amount_usd < 0.0 {
            fill_height
        } else {
            0.0
        };
        let bar_color = if flow.amount_usd == 0.0 {
            axis_color
        } else {
            signed_color(theme, flow.amount_usd)
        };
        let tooltip_text = format!(
            "{}\n{}",
            flow.date,
            format_signed_usd_amount(flow.amount_usd)
        );

        let bar = column![
            container(Space::new()).height(Length::Fixed(max_bar_height - positive_height)),
            flow_bar_segment(positive_height, bar_color),
            flow_bar_segment(1.0, axis_color),
            flow_bar_segment(negative_height, bar_color),
            container(Space::new()).height(Length::Fixed(max_bar_height - negative_height)),
        ]
        .width(Fill)
        .height(Length::Fixed(max_bar_height * 2.0 + 1.0));

        let wrapped_bar = tooltip(
            bar,
            text(tooltip_text).size(10).font(iced::Font::MONOSPACE),
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

        chart = chart.push(wrapped_bar);
    }

    container(chart)
        .width(Fill)
        .height(Length::Fixed(88.0))
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

fn fund_section(
    theme: &Theme,
    fund: &HypeEtfFund,
    available_width: f32,
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
        metric("Assets", format_usd_value(fund.net_assets_usd, 2), None),
        metric("HYPE", format_hype(fund.hype_exposure), None),
        metric("NAV", format_usd_value(fund.nav_per_share, 2), None),
        metric("Market", format_usd_value(fund.market_price, 2), None),
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
            format_usd_value(fund.hype_reference_price, 2),
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
                .font(iced::Font::MONOSPACE)
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

fn format_usd_value(value: Option<f64>, decimals: usize) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format_usd(&format!("{value:.decimals$}")))
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

fn format_signed_usd_amount(value: f64) -> String {
    let value = if value.abs() < 0.005 { 0.0 } else { value };
    let formatted = format_usd(&format!("{value:.2}"));
    if value > 0.0 {
        format!("+{formatted}")
    } else {
        formatted
    }
}

fn short_flow_date(date: &str) -> String {
    date.get(5..).unwrap_or(date).replace('-', "/")
}
