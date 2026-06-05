use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::positioning_state::{
    PositioningInfoChangeTimeframe, PositioningInfoInstance, PositioningInfoPage,
    PositioningInfoSide,
};

use buttons::{
    positioning_clear_filters_button, positioning_control_button, positioning_navigation_button,
};
use iced::widget::{Row, Space, container, row, text};
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

    pub(super) fn view_positioning_info_controls(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'static, Message> {
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
        row![
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
