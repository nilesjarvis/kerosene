use crate::app_state::TradingTerminal;
use crate::config::{
    DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY, DEFAULT_UI_SCALE, MAX_CHART_DOTTED_BACKGROUND_OPACITY,
    MAX_PANE_BORDER_THICKNESS, MAX_PANE_CORNER_RADIUS, MAX_UI_SCALE,
    MIN_CHART_DOTTED_BACKGROUND_OPACITY, MIN_PANE_BORDER_THICKNESS, MIN_PANE_CORNER_RADIUS,
    MIN_UI_SCALE, default_pane_border_thickness, default_pane_corner_radius,
};
use crate::message::Message;
use iced::widget::{checkbox, column, row, slider, text};
use iced::{Element, Fill, Length, Theme};
use std::ops::RangeInclusive;

// ---------------------------------------------------------------------------
// Widget Chrome Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_settings_widget_chrome_section(&self) -> Element<'_, Message> {
        let theme = self.theme();

        let mut content = column![
            text("Widget Chrome").size(14).color(theme.palette().text),
            scale_slider_row(
                &theme,
                "Scale",
                self.ui_scale,
                MIN_UI_SCALE..=MAX_UI_SCALE,
                Message::UiScaleChanged,
            ),
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
            checkbox(self.outer_widget_border_enabled)
                .label("Outer widget border")
                .on_toggle(Message::ToggleOuterWidgetBorder)
                .size(12)
                .spacing(8)
                .text_size(12)
                .font(crate::app_fonts::monospace_font()),
            checkbox(self.chart_dotted_background)
                .label("Dotted chart background")
                .on_toggle(Message::ToggleChartDottedBackground)
                .size(12)
                .spacing(8)
                .text_size(12)
                .font(crate::app_fonts::monospace_font()),
        ];

        if self.chart_dotted_background {
            content = content.push(opacity_slider_row(
                &theme,
                self.chart_dotted_background_opacity,
                MIN_CHART_DOTTED_BACKGROUND_OPACITY..=MAX_CHART_DOTTED_BACKGROUND_OPACITY,
                Message::ChartDottedBackgroundOpacityChanged,
            ));
        }

        content
            .push(
                text(format!(
                    "Defaults: {:.0}% scale, {:.0}% dots, {:.0}px divider, {:.0}px corners",
                    DEFAULT_UI_SCALE * 100.0,
                    DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY * 100.0,
                    default_pane_border_thickness(),
                    default_pane_corner_radius()
                ))
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            )
            .spacing(10)
            .into()
    }
}

fn scale_slider_row<'a>(
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
        slider(range, value, on_change).step(0.05).width(Fill),
        text(format!("{:.0}%", value * 100.0))
            .size(12)
            .color(theme.extended_palette().background.weak.text)
            .width(Length::Fixed(48.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .into()
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

fn opacity_slider_row<'a>(
    theme: &Theme,
    value: f32,
    range: RangeInclusive<f32>,
    on_change: fn(f32) -> Message,
) -> Element<'a, Message> {
    row![
        text("Opacity")
            .size(12)
            .color(theme.palette().text)
            .width(Length::Fixed(72.0)),
        slider(range, value, on_change).step(0.01).width(Fill),
        text(format!("{:.0}%", value * 100.0))
            .size(12)
            .color(theme.extended_palette().background.weak.text)
            .width(Length::Fixed(48.0)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .into()
}
