use crate::chart_state::{ChartId, ChartInstance};
use crate::config::{EmaCloudColor, EmaCloudPeriod, EmaCloudTimeframe, MAX_EMA_CLOUDS};
use crate::message::Message;
use iced::widget::{
    Column, Space, button, checkbox, pick_list, row, slider, text, text_input, tooltip,
};
use iced::{Alignment, Element, Fill, Length, Theme};

pub(super) fn view_ema_clouds(
    chart_id: ChartId,
    instance: &ChartInstance,
) -> Element<'static, Message> {
    let clouds = &instance.macro_indicators.ema_clouds;
    let add = button(text("+ Add").size(10))
        .on_press_maybe(
            (clouds.len() < MAX_EMA_CLOUDS).then_some(Message::ChartEmaCloudAdded(chart_id)),
        )
        .padding([2, 6])
        .style(button::text);
    let mut content = Column::new().spacing(5).width(Fill).push(
        row![text("EMA clouds").size(11), Space::new().width(Fill), add].align_y(Alignment::Center),
    );
    for cloud in clouds {
        let cloud_id = cloud.id;
        let period_input = |field, period: usize, label: &'static str| {
            let value = instance
                .ema_cloud_period_inputs
                .get(&(cloud_id, field))
                .cloned()
                .unwrap_or_else(|| period.to_string());
            let invalid = value.parse::<usize>().ok().is_none_or(|period| period == 0);
            tooltip(
                text_input(label, &value)
                    .on_input(move |value| {
                        Message::ChartEmaCloudPeriodChanged(chart_id, cloud_id, field, value)
                    })
                    .style(move |theme: &Theme, status| {
                        let mut style = text_input::default(theme, status);
                        if invalid {
                            style.border.color = theme.extended_palette().danger.base.color;
                        }
                        style
                    })
                    .padding([2, 4])
                    .size(10)
                    .width(Length::FillPortion(1)),
                text(format!("{label} EMA period: 1–5000")).size(11),
                tooltip::Position::Bottom,
            )
        };
        let toggle = checkbox(cloud.enabled)
            .on_toggle(move |_| Message::ChartEmaCloudToggled(chart_id, cloud_id))
            .size(12);
        let remove = tooltip(
            button(text("X").size(10))
                .on_press(Message::ChartEmaCloudRemoved(chart_id, cloud_id))
                .padding([2, 5])
                .style(button::text),
            text("Remove cloud").size(11),
            tooltip::Position::Bottom,
        );
        content = content
            .push(
                row![
                    toggle,
                    text("Fast").size(10),
                    period_input(EmaCloudPeriod::Fast, cloud.fast_period, "Fast"),
                    text("Slow").size(10),
                    period_input(EmaCloudPeriod::Slow, cloud.slow_period, "Slow"),
                    remove
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .push(
                row![
                    pick_list(
                        EmaCloudTimeframe::ALL,
                        Some(cloud.timeframe),
                        move |value| Message::ChartEmaCloudTimeframeChanged(
                            chart_id, cloud_id, value
                        )
                    )
                    .text_size(10)
                    .padding([2, 4])
                    .width(Length::FillPortion(1)),
                    pick_list(EmaCloudColor::ALL, Some(cloud.color), move |value| {
                        Message::ChartEmaCloudColorChanged(chart_id, cloud_id, value)
                    })
                    .text_size(10)
                    .padding([2, 4])
                    .width(Length::FillPortion(1)),
                ]
                .spacing(6),
            )
            .push(
                row![
                    text("Opacity").size(10),
                    slider(0..=100, cloud.opacity, move |value| {
                        Message::ChartEmaCloudOpacityChanged(chart_id, cloud_id, value)
                    })
                    .height(14),
                    text(format!("{}%", cloud.opacity)).size(10).width(32),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
    }
    content.into()
}
