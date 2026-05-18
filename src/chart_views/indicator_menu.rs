use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{
    Column, Space, button, checkbox, container, row, rule, scrollable, stack, text,
};
use iced::{Alignment, Color, Element, Fill, Font, Length, Theme};

// ---------------------------------------------------------------------------
// Indicator Menu Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct IndicatorOption {
    label: &'static str,
    key: &'static str,
    checked: bool,
}

impl TradingTerminal {
    pub(crate) fn view_macro_indicator_menu(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let indicator_options = &instance.macro_indicators;
        let separator = || compact_separator();

        let mut menu_col = Column::new()
            .spacing(3)
            .padding(6)
            .width(Fill)
            .push(indicator_group(
                chart_id,
                "TF",
                [
                    IndicatorOption {
                        label: "50 SMA",
                        key: "tf_sma_50",
                        checked: indicator_options.tf_sma_50,
                    },
                    IndicatorOption {
                        label: "50 EMA",
                        key: "tf_ema_50",
                        checked: indicator_options.tf_ema_50,
                    },
                    IndicatorOption {
                        label: "200 SMA",
                        key: "tf_sma_200",
                        checked: indicator_options.tf_sma_200,
                    },
                    IndicatorOption {
                        label: "200 EMA",
                        key: "tf_ema_200",
                        checked: indicator_options.tf_ema_200,
                    },
                ],
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "D",
                [
                    IndicatorOption {
                        label: "50 SMA",
                        key: "sma_50d",
                        checked: indicator_options.sma_50d,
                    },
                    IndicatorOption {
                        label: "50 EMA",
                        key: "ema_50d",
                        checked: indicator_options.ema_50d,
                    },
                    IndicatorOption {
                        label: "200 SMA",
                        key: "sma_200d",
                        checked: indicator_options.sma_200d,
                    },
                    IndicatorOption {
                        label: "200 EMA",
                        key: "ema_200d",
                        checked: indicator_options.ema_200d,
                    },
                ],
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "W",
                [
                    IndicatorOption {
                        label: "20 SMA",
                        key: "sma_20w",
                        checked: indicator_options.sma_20w,
                    },
                    IndicatorOption {
                        label: "20 EMA",
                        key: "ema_20w",
                        checked: indicator_options.ema_20w,
                    },
                    IndicatorOption {
                        label: "50 SMA",
                        key: "sma_50w",
                        checked: indicator_options.sma_50w,
                    },
                    IndicatorOption {
                        label: "50 EMA",
                        key: "ema_50w",
                        checked: indicator_options.ema_50w,
                    },
                ],
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "M",
                [
                    IndicatorOption {
                        label: "12 SMA",
                        key: "sma_12m",
                        checked: indicator_options.sma_12m,
                    },
                    IndicatorOption {
                        label: "12 EMA",
                        key: "ema_12m",
                        checked: indicator_options.ema_12m,
                    },
                ],
            ))
            .push(separator())
            .push(indicator_footer(
                chart_id,
                [
                    IndicatorOption {
                        label: "Funding",
                        key: "show_funding_rate",
                        checked: indicator_options.show_funding_rate,
                    },
                    IndicatorOption {
                        label: "Labels",
                        key: "show_labels",
                        checked: indicator_options.show_labels,
                    },
                ],
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "VOL",
                [IndicatorOption {
                    label: "Profile",
                    key: "show_volume_profile",
                    checked: indicator_options.show_volume_profile,
                }],
            ));

        if !instance.symbol.is_empty() && self.is_perp_coin(&instance.symbol) {
            menu_col = menu_col
                .push(separator())
                .push(overlay_group(chart_id, instance, &theme));
        }

        let menu_card = container(scrollable(menu_col).height(iced::Length::Shrink))
            .width(240.0)
            .max_height(220.0)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                border: iced::Border {
                    color: theme.extended_palette().background.weak.color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        let bg_overlay = button("")
            .width(Fill)
            .height(Fill)
            .on_press(Message::CloseAllMenus)
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            });

        stack![
            bg_overlay,
            container(menu_card)
                .width(Fill)
                .height(Fill)
                .padding([32, 20])
                .align_x(iced::Alignment::Start)
                .align_y(iced::Alignment::Start)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}

// ---------------------------------------------------------------------------
// Indicator Menu Components
// ---------------------------------------------------------------------------

fn indicator_group<const N: usize>(
    chart_id: ChartId,
    label: &'static str,
    options: [IndicatorOption; N],
) -> Element<'static, Message> {
    let mut rows = Column::new().spacing(2).width(Fill);

    for pair in options.chunks(2) {
        let mut option_row = row![].spacing(8).align_y(Alignment::Center).width(Fill);

        for option in pair {
            option_row = option_row.push(indicator_checkbox(chart_id, *option));
        }

        if pair.len() == 1 {
            option_row = option_row.push(Space::new().width(Length::FillPortion(1)));
        }

        rows = rows.push(option_row);
    }

    row![
        container(
            text(label)
                .size(10)
                .font(Font::MONOSPACE)
                .color(Color::from_rgb8(0x88, 0x88, 0x88))
        )
        .width(24.0),
        rows
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .width(Fill)
    .into()
}

fn indicator_footer<const N: usize>(
    chart_id: ChartId,
    options: [IndicatorOption; N],
) -> Element<'static, Message> {
    let mut option_row = row![].spacing(8).align_y(Alignment::Center).width(Fill);
    for option in options {
        option_row = option_row.push(indicator_checkbox(chart_id, option));
    }

    row![Space::new().width(24.0), option_row]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
}

fn indicator_checkbox(chart_id: ChartId, option: IndicatorOption) -> Element<'static, Message> {
    checkbox(option.checked)
        .label(option.label)
        .on_toggle(move |_| Message::ToggleMacroIndicator(chart_id, option.key.to_string()))
        .size(10)
        .spacing(4)
        .text_size(10)
        .font(Font::MONOSPACE)
        .width(Length::FillPortion(1))
        .into()
}

fn overlay_group(
    chart_id: ChartId,
    instance: &ChartInstance,
    theme: &Theme,
) -> Element<'static, Message> {
    let option_row = row![
        overlay_checkbox(
            "LIQ",
            instance.show_liquidations,
            Message::ToggleLiquidationOverlay(chart_id),
        ),
        overlay_checkbox(
            "HEAT",
            instance.show_heatmap,
            Message::ToggleHeatmapOverlay(chart_id),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Fill);

    let mut content = Column::new().spacing(2).width(Fill).push(option_row);

    if let Some(status) = overlay_status(instance, theme) {
        content = content.push(status);
    }

    row![
        container(
            text("OVR")
                .size(10)
                .font(Font::MONOSPACE)
                .color(Color::from_rgb8(0x88, 0x88, 0x88))
        )
        .width(24.0),
        content
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .width(Fill)
    .into()
}

fn overlay_checkbox(
    label: &'static str,
    checked: bool,
    message: Message,
) -> Element<'static, Message> {
    checkbox(checked)
        .label(label)
        .on_toggle(move |_| message.clone())
        .size(10)
        .spacing(4)
        .text_size(10)
        .font(Font::MONOSPACE)
        .width(Length::FillPortion(1))
        .into()
}

fn overlay_status(instance: &ChartInstance, theme: &Theme) -> Option<Element<'static, Message>> {
    let mut parts = Vec::new();
    let mut is_error = false;

    if instance.show_liquidations {
        if instance.liquidation_fetching {
            parts.push("LIQ loading".to_string());
        } else if let Some((status, status_is_error)) = &instance.liquidation_status {
            parts.push(status.clone());
            is_error |= *status_is_error;
        }
    }

    if instance.show_heatmap {
        if instance.heatmap_fetching {
            parts.push("HEAT loading".to_string());
        } else if let Some((status, status_is_error)) = &instance.heatmap_status {
            parts.push(status.clone());
            is_error |= *status_is_error;
        }
    }

    if parts.is_empty() {
        return None;
    }

    let color = if is_error {
        theme.palette().danger
    } else {
        theme.extended_palette().background.weak.text
    };

    Some(
        text(parts.join(" / "))
            .size(9)
            .font(Font::MONOSPACE)
            .color(color)
            .into(),
    )
}

fn compact_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.16,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}
