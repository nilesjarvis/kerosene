use components::{compact_separator, indicator_footer, indicator_group};
use options::{
    daily_options, footer_options, hourly_options, monthly_options, timeframe_options,
    volume_options, weekly_options,
};
use overlays::overlay_group;

use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, button, container, scrollable, stack};
use iced::{Color, Element, Fill, Theme};

mod components;
mod options;
mod overlays;

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
                timeframe_options(indicator_options),
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "1H",
                hourly_options(indicator_options),
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "D",
                daily_options(indicator_options),
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "W",
                weekly_options(indicator_options),
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "M",
                monthly_options(indicator_options),
            ))
            .push(separator())
            .push(indicator_footer(
                chart_id,
                footer_options(indicator_options),
            ))
            .push(separator())
            .push(indicator_group(
                chart_id,
                "VOL",
                volume_options(indicator_options),
            ));

        if !instance.symbol.is_empty() && self.is_perp_coin(&instance.symbol) {
            let earnings_available = self.chart_earnings_markers_available(instance);
            menu_col = menu_col.push(separator()).push(overlay_group(
                chart_id,
                instance,
                &theme,
                earnings_available,
            ));
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

#[cfg(test)]
mod tests;
