use crate::app_state::TradingTerminal;
use crate::chart::crosshair_style::{CrosshairStyleRender, draw_crosshair_style};
use crate::config::{
    ChartCrosshairStyle, ChartHudOrderSound, DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH,
    DEFAULT_CHART_CROSSHAIR_SCALE, DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY,
    DEFAULT_CHART_EDGE_BLUR_STRENGTH, DEFAULT_CHART_FISHEYE_STRENGTH,
    DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME, DEFAULT_UI_SCALE,
    MAX_CHART_CHROMATIC_ABERRATION_STRENGTH, MAX_CHART_CROSSHAIR_SCALE,
    MAX_CHART_DOTTED_BACKGROUND_OPACITY, MAX_CHART_EDGE_BLUR_STRENGTH, MAX_CHART_FISHEYE_STRENGTH,
    MAX_CHART_HUD_ORDER_SOUND_VOLUME, MAX_PANE_BORDER_THICKNESS, MAX_PANE_CORNER_RADIUS,
    MAX_UI_SCALE, MIN_CHART_CHROMATIC_ABERRATION_STRENGTH, MIN_CHART_CROSSHAIR_SCALE,
    MIN_CHART_DOTTED_BACKGROUND_OPACITY, MIN_CHART_EDGE_BLUR_STRENGTH, MIN_CHART_FISHEYE_STRENGTH,
    MIN_CHART_HUD_ORDER_SOUND_VOLUME, MIN_PANE_BORDER_THICKNESS, MIN_PANE_CORNER_RADIUS,
    MIN_UI_SCALE, default_pane_border_thickness, default_pane_corner_radius,
};
use crate::message::Message;
use iced::widget::{Column, Row, button, checkbox, column, pick_list, row, slider, text};
use iced::{Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme};
use std::ops::RangeInclusive;

// ---------------------------------------------------------------------------
// Widget Chrome Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_settings_widget_chrome_section(&self) -> Element<'_, Message> {
        let theme = self.theme();

        let mut content = column![
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
            checkbox(self.chart_fisheye_enabled)
                .label("Chart fisheye lens")
                .on_toggle(Message::ToggleChartFisheye)
                .size(12)
                .spacing(8)
                .text_size(12)
                .font(crate::app_fonts::monospace_font()),
            checkbox(self.chart_chromatic_aberration_enabled)
                .label("Chart chromatic aberration")
                .on_toggle(Message::ToggleChartChromaticAberration)
                .size(12)
                .spacing(8)
                .text_size(12)
                .font(crate::app_fonts::monospace_font()),
            checkbox(self.chart_edge_blur_enabled)
                .label("Chart edge blur")
                .on_toggle(Message::ToggleChartEdgeBlur)
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

        if self.chart_fisheye_enabled {
            content = content.push(scale_slider_row(
                &theme,
                "Lens",
                self.chart_fisheye_strength,
                MIN_CHART_FISHEYE_STRENGTH..=MAX_CHART_FISHEYE_STRENGTH,
                Message::ChartFisheyeStrengthChanged,
            ));
        }

        if self.chart_chromatic_aberration_enabled {
            content = content.push(scale_slider_row(
                &theme,
                "Fringe",
                self.chart_chromatic_aberration_strength,
                MIN_CHART_CHROMATIC_ABERRATION_STRENGTH..=MAX_CHART_CHROMATIC_ABERRATION_STRENGTH,
                Message::ChartChromaticAberrationStrengthChanged,
            ));
        }

        if self.chart_edge_blur_enabled {
            content = content.push(scale_slider_row(
                &theme,
                "Blur",
                self.chart_edge_blur_strength,
                MIN_CHART_EDGE_BLUR_STRENGTH..=MAX_CHART_EDGE_BLUR_STRENGTH,
                Message::ChartEdgeBlurStrengthChanged,
            ));
        }

        content
            .push(
                text(format!(
                    "Defaults: {:.0}% scale, {:.0}% dots, {:.0}% lens, {:.0}% fringe, {:.0}% blur, {:.0}px divider, {:.0}px corners",
                    DEFAULT_UI_SCALE * 100.0,
                    DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY * 100.0,
                    DEFAULT_CHART_FISHEYE_STRENGTH * 100.0,
                    DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH * 100.0,
                    DEFAULT_CHART_EDGE_BLUR_STRENGTH * 100.0,
                    default_pane_border_thickness(),
                    default_pane_corner_radius()
                ))
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            )
            .spacing(10)
            .into()
    }

    pub(super) fn view_settings_crosshair_section(&self) -> Element<'_, Message> {
        let theme = self.theme();

        column![
            checkbox(self.chart_crosshair_guides_enabled)
                .label("Full-span guide lines")
                .on_toggle(Message::ToggleChartCrosshairGuides)
                .size(12)
                .spacing(8)
                .text_size(12)
                .font(crate::app_fonts::monospace_font()),
            scale_slider_row(
                &theme,
                "Size",
                self.chart_crosshair_scale,
                MIN_CHART_CROSSHAIR_SCALE..=MAX_CHART_CROSSHAIR_SCALE,
                Message::ChartCrosshairScaleChanged,
            ),
            crosshair_style_grid(
                &theme,
                self.chart_crosshair_style,
                self.chart_crosshair_guides_enabled,
                self.chart_crosshair_scale,
            ),
            hud_order_sound_settings(
                &theme,
                self.chart_hud_order_sound,
                self.chart_hud_order_sound_file.as_deref(),
                self.chart_hud_order_sound_volume,
            ),
            text(format!(
                "Defaults: {} style, full-span guides on, {:.0}% size, {} HUD sound, {:.0}% HUD volume",
                ChartCrosshairStyle::default().label(),
                DEFAULT_CHART_CROSSHAIR_SCALE * 100.0,
                ChartHudOrderSound::default().label(),
                DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME * 100.0,
            ))
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(10)
        .into()
    }
}

fn hud_order_sound_settings<'a>(
    theme: &Theme,
    selected: ChartHudOrderSound,
    custom_file: Option<&'a str>,
    volume: f32,
) -> Element<'a, Message> {
    let custom_label = custom_file.unwrap_or("No custom WAV imported");

    column![
        text("HUD order sound").size(13).color(theme.palette().text),
        row![
            pick_list(
                ChartHudOrderSound::ALL.to_vec(),
                Some(selected),
                Message::ChartHudOrderSoundChanged,
            )
            .padding([4, 8])
            .text_size(12)
            .width(Length::Fixed(150.0)),
            button(text("Import WAV").size(12)).on_press(Message::ImportChartHudOrderSound),
            button(text("Test").size(12)).on_press(Message::TestChartHudOrderSound),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        scale_slider_row(
            theme,
            "Volume",
            volume,
            MIN_CHART_HUD_ORDER_SOUND_VOLUME..=MAX_CHART_HUD_ORDER_SOUND_VOLUME,
            Message::ChartHudOrderSoundVolumeChanged,
        ),
        text(custom_label)
            .size(11)
            .color(theme.extended_palette().background.weak.text),
    ]
    .spacing(7)
    .into()
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

fn crosshair_style_grid(
    theme: &Theme,
    selected: ChartCrosshairStyle,
    guide_lines_enabled: bool,
    crosshair_scale: f32,
) -> Element<'static, Message> {
    let mut grid = Column::new().spacing(6).width(Fill);

    for styles in ChartCrosshairStyle::ALL.chunks(2) {
        let mut style_row = Row::new().spacing(6).width(Fill);
        for style in styles {
            style_row = style_row.push(crosshair_style_card(
                theme,
                *style,
                selected,
                guide_lines_enabled,
                crosshair_scale,
            ));
        }
        grid = grid.push(style_row);
    }

    grid.into()
}

fn crosshair_style_card(
    theme: &Theme,
    style: ChartCrosshairStyle,
    selected: ChartCrosshairStyle,
    guide_lines_enabled: bool,
    crosshair_scale: f32,
) -> Element<'static, Message> {
    let is_selected = style == selected;
    let label_color = if is_selected {
        theme.palette().primary
    } else {
        theme.extended_palette().background.weak.text
    };

    let preview: Element<'static, Message> = iced::widget::canvas(CrosshairStylePreview {
        style,
        guide_lines_enabled,
        crosshair_scale,
    })
    .width(Fill)
    .height(Length::Fixed(38.0))
    .into();

    let content = column![
        preview,
        text(style.label())
            .size(10)
            .color(label_color)
            .font(crate::app_fonts::monospace_font())
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .width(Fill);

    button(content)
        .on_press(Message::ChartCrosshairStyleChanged(style))
        .padding([6, 7])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if is_selected => Color {
                    a: 0.38,
                    ..theme.extended_palette().background.strong.color
                },
                _ => Color {
                    a: 0.22,
                    ..theme.extended_palette().background.weak.color
                },
            };
            let border_color = if is_selected {
                theme.palette().primary
            } else {
                Color {
                    a: 0.28,
                    ..theme.extended_palette().background.weak.text
                }
            };

            button::Style {
                background: Some(background.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    color: border_color,
                    width: if is_selected { 1.0 } else { 0.5 },
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

struct CrosshairStylePreview {
    style: ChartCrosshairStyle,
    guide_lines_enabled: bool,
    crosshair_scale: f32,
}

impl iced::widget::canvas::Program<Message> for CrosshairStylePreview {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let border_size = Size::new(
            (bounds.width - 1.0).max(0.0),
            (bounds.height - 1.0).max(0.0),
        );
        let border = iced::widget::canvas::Path::rectangle(Point::new(0.5, 0.5), border_size);
        frame.stroke(
            &border,
            iced::widget::canvas::Stroke::default()
                .with_color(Color {
                    a: 0.14,
                    ..theme.extended_palette().background.weak.text
                })
                .with_width(1.0),
        );

        draw_crosshair_style(
            &mut frame,
            theme,
            CrosshairStyleRender {
                style: self.style,
                guide_lines_enabled: self.guide_lines_enabled,
                crosshair_scale: self.crosshair_scale,
                position: Point::new(bounds.width * 0.5, bounds.height * 0.5),
                width: bounds.width,
                height: bounds.height,
                fisheye: crate::chart::fisheye::ChartFisheye::disabled(),
                accent_color: None,
            },
        );

        vec![frame.into_geometry()]
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
