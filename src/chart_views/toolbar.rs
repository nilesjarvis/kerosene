mod sections;

use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use crate::timeframe::TIMEFRAME_OPTIONS;
use iced::Element;
use iced::widget::{pick_list, row};

impl TradingTerminal {
    pub(crate) fn view_chart_toolbar(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let has_candles = !instance.chart.candles.is_empty();
        let active = instance.interval;
        let active_tool = instance.chart.active_tool;
        let tf_picker = pick_list(TIMEFRAME_OPTIONS, Some(active), move |tf| {
            Message::ChartSwitchTimeframe(chart_id, tf)
        })
        .width(iced::Length::Shrink)
        .padding([2, 8])
        .text_size(11);

        let indicator_btn =
            self.view_chart_indicator_button(chart_id, instance.macro_menu_open, &theme);
        let reload_btn = sections::chart_reload_button(chart_id);
        let reset_view_btn = sections::chart_reset_view_button(chart_id);

        let mut tf_row = row![tf_picker, indicator_btn, reload_btn, reset_view_btn]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        if let Some(status) = sections::chart_fetch_status_label(has_candles, instance, &theme) {
            tf_row = tf_row.push(status);
        }

        tf_row = sections::push_drawing_tool_buttons(tf_row, chart_id, active_tool);
        tf_row = sections::push_chart_mode_buttons(tf_row, chart_id, instance);

        let is_perp_chart = !instance.symbol.is_empty() && self.is_perp_coin(&instance.symbol);
        let heatmap_spinner = instance
            .heatmap_fetching
            .then(|| self.view_inline_spinner(12));
        tf_row = sections::push_market_overlay_buttons(
            tf_row,
            chart_id,
            instance,
            is_perp_chart,
            heatmap_spinner,
        );

        tf_row.wrap().into()
    }
}
