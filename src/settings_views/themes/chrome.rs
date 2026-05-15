use crate::app_state::TradingTerminal;
use crate::config::{
    MAX_PANE_BORDER_THICKNESS, MAX_PANE_CORNER_RADIUS, MIN_PANE_BORDER_THICKNESS,
    MIN_PANE_CORNER_RADIUS, default_pane_border_thickness, default_pane_corner_radius,
};
use crate::message::Message;
use iced::widget::{column, row, slider, text};
use iced::{Element, Fill, Length, Theme};
use std::ops::RangeInclusive;

// ---------------------------------------------------------------------------
// Widget Chrome Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_settings_widget_chrome_section(&self) -> Element<'_, Message> {
        let theme = self.theme();

        column![
            text("Widget Chrome").size(14).color(theme.palette().text),
            chrome_slider_row(
                &theme,
                "Divider",
                self.pane_border_thickness,
                MIN_PANE_BORDER_THICKNESS..=MAX_PANE_BORDER_THICKNESS,
                Message::PaneBorderThicknessChanged,
            ),
            chrome_slider_row(
                &theme,
                "Corners",
                self.pane_corner_radius,
                MIN_PANE_CORNER_RADIUS..=MAX_PANE_CORNER_RADIUS,
                Message::PaneCornerRadiusChanged,
            ),
            text(format!(
                "Defaults: {:.0}px divider, {:.0}px corners",
                default_pane_border_thickness(),
                default_pane_corner_radius()
            ))
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(10)
        .into()
    }
}

fn chrome_slider_row<'a>(
    theme: &Theme,
    label: &'static str,
    value: f32,
    range: RangeInclusive<f32>,
    on_change: fn(f32) -> Message,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .color(theme.palette().text)
            .width(Length::Fixed(72.0)),
        slider(range, value, on_change).step(1.0).width(Fill),
        text(format!("{value:.0}px"))
            .size(12)
            .color(theme.extended_palette().background.weak.text)
            .width(Length::Fixed(48.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .into()
}
