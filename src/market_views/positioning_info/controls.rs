use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::positioning_state::{
    PositioningInfoChangeTimeframe, PositioningInfoInstance, PositioningInfoPage,
    PositioningInfoSide,
};

use buttons::{
    positioning_clear_filters_button, positioning_control_button, positioning_navigation_button,
};
use iced::widget::{Column, Row, Space, container, row, text, text_input};
use iced::{Alignment, Element, Fill, Length};

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
        let nav = PositioningInfoPage::ALL.iter().fold(
            Row::new().spacing(4).align_y(Alignment::Center),
            |row, &page| {
                row.push(positioning_navigation_button(
                    instance.id,
                    page,
                    instance.page == page,
                ))
            },
        );

        container(nav).width(Fill).padding([8, 10]).into()
    }

    pub(super) fn view_positioning_info_controls<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
    ) -> Element<'a, Message> {
        let muted = self.theme().extended_palette().background.weak.text;
        let can_clear = instance.has_active_filters() || instance.error.is_some();
        let side_row = PositioningInfoSide::ALL
            .iter()
            .fold(Row::new().spacing(4), |row, &side| {
                row.push(positioning_control_button(
                    side.label(),
                    instance.side == side,
                    Message::PositioningInfoSideChanged(instance.id, side),
                ))
            });
        let side_controls = row![
            text("Side")
                .size(10)
                .color(muted)
                .width(Length::Fixed(34.0)),
            side_row,
            Space::new().width(Fill),
            positioning_clear_filters_button(instance.id, can_clear),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill);

        let has_entry_range = !instance.entry_min_input.trim().is_empty()
            || !instance.entry_max_input.trim().is_empty();
        let entry_min = text_input("Min", &instance.entry_min_input)
            .style(helpers::text_input_style)
            .on_input(move |value| Message::PositioningInfoEntryMinChanged(instance.id, value))
            .on_submit(Message::ApplyPositioningInfoEntryRange(instance.id))
            .size(10)
            .padding([2, 6])
            .width(Length::Fixed(72.0));
        let entry_max = text_input("Max", &instance.entry_max_input)
            .style(helpers::text_input_style)
            .on_input(move |value| Message::PositioningInfoEntryMaxChanged(instance.id, value))
            .on_submit(Message::ApplyPositioningInfoEntryRange(instance.id))
            .size(10)
            .padding([2, 6])
            .width(Length::Fixed(72.0));
        let entry_controls = row![
            text("Entry")
                .size(10)
                .color(muted)
                .width(Length::Fixed(34.0)),
            entry_min,
            text("-").size(10).color(muted),
            entry_max,
            positioning_control_button(
                "Apply",
                has_entry_range,
                Message::ApplyPositioningInfoEntryRange(instance.id),
            ),
            Space::new().width(Fill),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill);

        Column::new()
            .spacing(5)
            .push(side_controls)
            .push(entry_controls)
            .width(Fill)
            .into()
    }

    pub(super) fn view_positioning_info_change_controls(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'static, Message> {
        let muted = self.theme().extended_palette().background.weak.text;
        let timeframe_row = PositioningInfoChangeTimeframe::ALL.iter().fold(
            Row::new().spacing(4),
            |row, &timeframe| {
                row.push(positioning_control_button(
                    timeframe.label(),
                    instance.change_timeframe == timeframe,
                    Message::PositioningInfoChangeTimeframeChanged(instance.id, timeframe),
                ))
            },
        );

        row![
            text("Time")
                .size(10)
                .color(muted)
                .width(Length::Fixed(34.0)),
            timeframe_row,
            Space::new().width(Fill),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
    }
}
