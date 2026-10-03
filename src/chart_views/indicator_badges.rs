use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;

use self::active::active_chart_indicators;
use self::badge::{indicator_badge, indicator_badge_with_action};
use iced::widget::{column, container};
use iced::{Alignment, Element, Length};

mod active;
mod badge;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Chart Indicator Badges
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_chart_indicator_badges(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Option<Element<'static, Message>> {
        let theme = self.theme();
        let active_indicators = active_chart_indicators(instance, &theme);
        if active_indicators.is_empty()
            && !instance
                .macro_indicators
                .ema_clouds
                .iter()
                .any(|cloud| cloud.enabled)
        {
            return None;
        }

        let mut badges = column![].spacing(4).align_x(Alignment::Start);
        for indicator in active_indicators {
            badges = badges.push(indicator_badge(chart_id, indicator));
        }

        for cloud in instance
            .macro_indicators
            .ema_clouds
            .iter()
            .filter(|cloud| cloud.enabled)
        {
            badges = badges.push(indicator_badge_with_action(
                cloud.label(),
                cloud.color.color(&theme, true),
                Message::ChartEmaCloudToggled(chart_id, cloud.id),
            ));
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
