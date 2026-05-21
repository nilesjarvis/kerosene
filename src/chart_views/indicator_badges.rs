use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, text, tooltip};
use iced::{Alignment, Color, Element, Length, Theme};

const REMOVE_ICON: &str = "X";

// ---------------------------------------------------------------------------
// Chart Indicator Badges
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct ActiveIndicator {
    label: &'static str,
    key: &'static str,
    color: Color,
}

#[derive(Debug, Clone, Copy)]
enum IndicatorColorRole {
    Fast,
    Slow,
    WeeklyFast,
    WeeklySlow,
    Monthly,
    Funding,
    VolumeProfile,
}

impl IndicatorColorRole {
    fn color(self, theme: &Theme) -> Color {
        let extended = theme.extended_palette();

        match self {
            Self::Fast => extended.warning.base.color,
            Self::Slow => extended.primary.base.color,
            Self::WeeklyFast => extended.success.base.color,
            Self::WeeklySlow | Self::Funding => extended.secondary.strong.color,
            Self::Monthly => extended.danger.base.color,
            Self::VolumeProfile => theme.palette().primary,
        }
    }
}

impl TradingTerminal {
    pub(crate) fn view_chart_indicator_badges(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Option<Element<'static, Message>> {
        let theme = self.theme();
        let active_indicators = active_chart_indicators(instance, &theme);
        if active_indicators.is_empty() {
            return None;
        }

        let mut badges = column![].spacing(4).align_x(Alignment::Start);
        for indicator in active_indicators {
            badges = badges.push(indicator_badge(chart_id, indicator));
        }

        Some(
            container(badges.wrap())
                .padding(iced::Padding {
                    top: 8.0,
                    right: 8.0,
                    bottom: 0.0,
                    left: 8.0,
                })
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into(),
        )
    }
}

fn active_chart_indicators(instance: &ChartInstance, theme: &Theme) -> Vec<ActiveIndicator> {
    let indicators = &instance.macro_indicators;
    let mut active = Vec::new();

    push_indicator(
        &mut active,
        indicators.tf_sma_50,
        "TF 50 SMA",
        "tf_sma_50",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_ema_50,
        "TF 50 EMA",
        "tf_ema_50",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_sma_200,
        "TF 200 SMA",
        "tf_sma_200",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.tf_ema_200,
        "TF 200 EMA",
        "tf_ema_200",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50d,
        "50d SMA",
        "sma_50d",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50d,
        "50d EMA",
        "ema_50d",
        IndicatorColorRole::Fast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_200d,
        "200d SMA",
        "sma_200d",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_200d,
        "200d EMA",
        "ema_200d",
        IndicatorColorRole::Slow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_20w,
        "20w SMA",
        "sma_20w",
        IndicatorColorRole::WeeklyFast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_20w,
        "20w EMA",
        "ema_20w",
        IndicatorColorRole::WeeklyFast,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_50w,
        "50w SMA",
        "sma_50w",
        IndicatorColorRole::WeeklySlow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_50w,
        "50w EMA",
        "ema_50w",
        IndicatorColorRole::WeeklySlow,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.sma_12m,
        "12M SMA",
        "sma_12m",
        IndicatorColorRole::Monthly,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.ema_12m,
        "12M EMA",
        "ema_12m",
        IndicatorColorRole::Monthly,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_funding_rate,
        "Funding",
        "show_funding_rate",
        IndicatorColorRole::Funding,
        theme,
    );
    push_indicator(
        &mut active,
        indicators.show_volume_profile,
        "Vol Profile",
        "show_volume_profile",
        IndicatorColorRole::VolumeProfile,
        theme,
    );

    active
}

fn push_indicator(
    active: &mut Vec<ActiveIndicator>,
    enabled: bool,
    label: &'static str,
    key: &'static str,
    color_role: IndicatorColorRole,
    theme: &Theme,
) {
    if enabled {
        active.push(ActiveIndicator {
            label,
            key,
            color: color_role.color(theme),
        });
    }
}

fn indicator_badge(chart_id: ChartId, indicator: ActiveIndicator) -> Element<'static, Message> {
    let swatch = container(Space::new().width(6.0).height(6.0))
        .width(6.0)
        .height(6.0)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(indicator.color.into()),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let badge = button(
        row![
            swatch,
            text(indicator.label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(indicator.color),
            text(REMOVE_ICON).size(10).font(crate::app_fonts::monospace_font()),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ToggleMacroIndicator(
        chart_id,
        indicator.key.to_string(),
    ))
    .padding([2, 6])
    .style(move |theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => theme.extended_palette().background.strong.color,
            _ => Color {
                a: 0.86,
                ..theme.extended_palette().background.base.color
            },
        };

        button::Style {
            background: Some(bg.into()),
            text_color: theme.palette().text,
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.5,
                    ..indicator.color
                },
            },
            ..Default::default()
        }
    });

    tooltip(
        badge,
        text(format!("Remove {}", indicator.label))
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Bottom,
    )
    .into()
}
