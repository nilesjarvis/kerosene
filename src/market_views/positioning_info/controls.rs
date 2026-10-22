use crate::app_state::TradingTerminal;
use crate::helpers::design::{UiColors, compact_input_style};
use crate::message::Message;
use crate::positioning_state::{
    PositioningInfoChangeTimeframe, PositioningInfoInstance, PositioningInfoPage,
    PositioningInfoSide,
};

use buttons::{
    positioning_clear_filters_button, positioning_control_button, positioning_filler,
    positioning_navigation_button, positioning_segment_track,
};
use iced::widget::text::LineHeight;
use iced::widget::{Column, Row, container, row, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme};

mod buttons;
mod symbol_picker;

// ---------------------------------------------------------------------------
// Positioning Information Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_positioning_info_navigation(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'static, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let nav = PositioningInfoPage::ALL.iter().fold(
            iced::widget::Row::new()
                .spacing(2)
                .align_y(Alignment::Center),
            |row, &page| {
                row.push(positioning_navigation_button(
                    instance.id,
                    page,
                    instance.page == page,
                    pane_corner_radius,
                ))
            },
        );

        container(nav).width(Fill).padding([8, 10]).into()
    }

    pub(super) fn view_positioning_info_controls<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
    ) -> Element<'a, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let can_clear = instance.has_active_filters() || instance.error.is_some();
        let side_row = PositioningInfoSide::ALL
            .iter()
            .fold(Row::new().spacing(2), |row, &side| {
                row.push(positioning_control_button(
                    side.label(),
                    instance.side == side,
                    Message::PositioningInfoSideChanged(instance.id, side),
                    pane_corner_radius,
                ))
            });
        let side_controls = row![
            text("Side")
                .size(11)
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).muted),
                })
                .width(Length::Fixed(34.0)),
            positioning_segment_track(side_row, pane_corner_radius),
            positioning_filler(),
            positioning_clear_filters_button(instance.id, can_clear, pane_corner_radius),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill);

        let has_entry_range = !instance.entry_min_input.trim().is_empty()
            || !instance.entry_max_input.trim().is_empty();
        let entry_min = text_input("Min", &instance.entry_min_input)
            .on_input(move |value| Message::PositioningInfoEntryMinChanged(instance.id, value))
            .on_submit(Message::ApplyPositioningInfoEntryRange(instance.id))
            .size(12)
            .line_height(LineHeight::Absolute(iced::Pixels(16.0)))
            .padding([4, 8])
            .width(Length::Fixed(72.0))
            .style(move |theme, status| {
                compact_input_style(theme, status, false, pane_corner_radius)
            });
        let entry_max = text_input("Max", &instance.entry_max_input)
            .on_input(move |value| Message::PositioningInfoEntryMaxChanged(instance.id, value))
            .on_submit(Message::ApplyPositioningInfoEntryRange(instance.id))
            .size(12)
            .line_height(LineHeight::Absolute(iced::Pixels(16.0)))
            .padding([4, 8])
            .width(Length::Fixed(72.0))
            .style(move |theme, status| {
                compact_input_style(theme, status, false, pane_corner_radius)
            });
        let entry_controls = row![
            text("Entry")
                .size(11)
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).muted),
                })
                .width(Length::Fixed(34.0)),
            entry_min,
            text("-").size(11).style(move |theme: &Theme| text::Style {
                color: Some(UiColors::new(theme).muted),
            }),
            entry_max,
            positioning_control_button(
                "Apply",
                has_entry_range,
                Message::ApplyPositioningInfoEntryRange(instance.id),
                pane_corner_radius,
            ),
            positioning_filler(),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill);

        Column::new()
            .spacing(4)
            .push(side_controls)
            .push(entry_controls)
            .width(Fill)
            .into()
    }

    pub(super) fn view_positioning_info_change_controls(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'static, Message> {
        let pane_corner_radius = self.pane_corner_radius;
        let timeframe_row = PositioningInfoChangeTimeframe::ALL.iter().fold(
            Row::new().spacing(2),
            |row, &timeframe| {
                row.push(positioning_control_button(
                    timeframe.label(),
                    instance.change_timeframe == timeframe,
                    Message::PositioningInfoChangeTimeframeChanged(instance.id, timeframe),
                    pane_corner_radius,
                ))
            },
        );

        row![
            text("Time")
                .size(11)
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).muted),
                })
                .width(Length::Fixed(34.0)),
            positioning_segment_track(timeframe_row, pane_corner_radius),
            positioning_filler(),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
    }
}
