mod sections;

use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use crate::timeframe::TIMEFRAME_OPTIONS;
use iced::widget::{pick_list, row};
use iced::{Color, Element, Length, Theme};

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
        .width(Length::Shrink)
        .padding([3, 8])
        .text_size(11)
        .style(chart_toolbar_pick_list_style);

        let indicator_btn = sections::chart_toolbar_button(
            "IND",
            instance.macro_menu_open,
            Message::ToggleMacroMenu(chart_id),
        );
        let reload_btn = sections::chart_reload_button(chart_id);
        let reset_view_btn = sections::chart_reset_view_button(chart_id);

        let mut tf_row = row![
            tf_picker,
            sections::chart_toolbar_separator(),
            indicator_btn,
            sections::chart_toolbar_separator(),
            reload_btn,
            sections::chart_toolbar_separator(),
            reset_view_btn,
        ]
        .spacing(0)
        .align_y(iced::Alignment::Center);

        if let Some(status) = sections::chart_fetch_status_label(has_candles, instance, &theme) {
            tf_row = tf_row.push(sections::chart_toolbar_separator()).push(status);
        }

        tf_row = sections::push_drawing_tool_buttons(tf_row, chart_id, active_tool);
        tf_row = sections::push_chart_mode_buttons(tf_row, chart_id, instance);

        sections::chart_toolbar_strip(tf_row)
    }
}

fn chart_toolbar_pick_list_style(
    theme: &Theme,
    status: pick_list::Status,
) -> pick_list::Style {
    let background = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => Color {
            a: 0.55,
            ..theme.extended_palette().background.strong.color
        },
        pick_list::Status::Active => Color::TRANSPARENT,
    };

    pick_list::Style {
        text_color: theme.extended_palette().background.weak.text,
        placeholder_color: theme.extended_palette().background.weak.text,
        handle_color: theme.extended_palette().background.weak.text,
        background: background.into(),
        border: iced::Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}
