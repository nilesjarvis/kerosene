use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, row, text};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(in crate::feed_views::liquidations::controls) fn view_liquidation_display_toggles(
        &self,
    ) -> Element<'_, Message> {
        let alerts_btn = liquidation_toggle_button(
            if self.liquidation_alerts_enabled {
                "Alerts: ON"
            } else {
                "Alerts: OFF"
            },
            self.liquidation_alerts_enabled,
            false,
            Message::ToggleLiquidationAlerts,
        );

        let chart_btn = liquidation_toggle_button(
            if self.liquidation_chart_enabled {
                "Chart: ON"
            } else {
                "Chart: OFF"
            },
            self.liquidation_chart_enabled,
            false,
            Message::ToggleLiquidationChart,
        );

        let summary_btn = liquidation_toggle_button(
            if self.liquidation_summary_enabled {
                "Summary: ON"
            } else {
                "Summary: OFF"
            },
            self.liquidation_summary_enabled,
            false,
            Message::ToggleLiquidationSummary,
        );

        let aggregation_btn = liquidation_toggle_button(
            if self.liquidation_feed_aggregation_enabled {
                "Rows: Positions"
            } else {
                "Rows: Fills"
            },
            self.liquidation_feed_aggregation_enabled,
            true,
            Message::ToggleLiquidationFeedAggregation,
        );

        row![chart_btn, summary_btn, aggregation_btn, alerts_btn]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    }
}

fn liquidation_toggle_button(
    label: &'static str,
    enabled: bool,
    primary_when_enabled: bool,
    message: Message,
) -> Element<'static, Message> {
    button(text(label).size(10))
        .on_press(message)
        .padding([2, 6])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if enabled {
                    if primary_when_enabled {
                        theme.palette().primary
                    } else {
                        theme.palette().success
                    }
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}
