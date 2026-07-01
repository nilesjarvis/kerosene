use crate::app_state::TradingTerminal;
use crate::chart::crosshair_style::{CrosshairStyleRender, RacingHudMetrics, draw_crosshair_style};
use crate::config::{
    ChartCrosshairStyle, ChartHollowCandleMode, ChartHudOrderSound, ChartHudReadoutConfig,
    ChartHudReadoutElement, ChartSeriesStyle, DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH,
    DEFAULT_CHART_CROSSHAIR_SCALE, DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY,
    DEFAULT_CHART_EDGE_BLUR_STRENGTH, DEFAULT_CHART_FISHEYE_STRENGTH,
    DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME, DEFAULT_UI_SCALE,
    MAX_CHART_CHROMATIC_ABERRATION_STRENGTH, MAX_CHART_CROSSHAIR_SCALE,
    MAX_CHART_DOTTED_BACKGROUND_OPACITY, MAX_CHART_EDGE_BLUR_STRENGTH, MAX_CHART_FISHEYE_STRENGTH,
    MAX_CHART_HUD_ORDER_SOUND_VOLUME, MAX_PANE_BORDER_THICKNESS, MAX_PANE_CORNER_RADIUS,
    MAX_UI_SCALE, MAX_WIDGET_PADDING, MIN_CHART_CHROMATIC_ABERRATION_STRENGTH,
    MIN_CHART_CROSSHAIR_SCALE, MIN_CHART_DOTTED_BACKGROUND_OPACITY, MIN_CHART_EDGE_BLUR_STRENGTH,
    MIN_CHART_FISHEYE_STRENGTH, MIN_CHART_HUD_ORDER_SOUND_VOLUME, MIN_PANE_BORDER_THICKNESS,
    MIN_PANE_CORNER_RADIUS, MIN_UI_SCALE, MIN_WIDGET_PADDING, default_pane_border_thickness,
    default_pane_corner_radius, default_widget_padding,
};
use crate::helpers::pane_title;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{
    Column, Row, Space, button, checkbox, column, container, pick_list, row, rule, slider, text,
};
use iced::{Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme};
use std::ops::RangeInclusive;

// ---------------------------------------------------------------------------
// Widget Chrome Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_settings_widget_chrome_section(&self) -> Element<'_, Message> {
        let theme = self.theme();

        let interface_controls = column![
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
            chrome_slider_row(
                &theme,
                "Padding",
                self.widget_padding_default,
                MIN_WIDGET_PADDING..=MAX_WIDGET_PADDING,
                Message::DefaultWidgetPaddingChanged,
            ),
            self.view_focused_widget_padding_controls(&theme),
        ]
        .spacing(9);

        let window_controls = column![
            toggle_status_row(
                &theme,
                "Outer widget border",
                self.outer_widget_border_enabled,
                Message::ToggleOuterWidgetBorder,
            ),
            toggle_status_row(
                &theme,
                "Custom OS bar",
                self.custom_window_chrome_enabled,
                Message::ToggleCustomWindowChrome,
            ),
        ]
        .spacing(8);

        let mut background_controls = column![
            toggle_status_row(
                &theme,
                "Dotted chart background",
                self.chart_dotted_background,
                Message::ToggleChartDottedBackground,
            ),
            toggle_status_row(
                &theme,
                "Gradient chart background",
                self.chart_gradient_background,
                Message::ToggleChartGradientBackground,
            ),
        ]
        .spacing(8);

        if self.chart_dotted_background {
            background_controls = background_controls.push(opacity_slider_row(
                &theme,
                self.chart_dotted_background_opacity,
                MIN_CHART_DOTTED_BACKGROUND_OPACITY..=MAX_CHART_DOTTED_BACKGROUND_OPACITY,
                Message::ChartDottedBackgroundOpacityChanged,
            ));
        }

        let chart_style_controls = column![
            row![
                column![
                    text("Chart style").size(12).color(theme.palette().text),
                    text(self.chart_series_style.label())
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(2)
                .width(Fill),
                pick_list(
                    ChartSeriesStyle::ALL.to_vec(),
                    Some(self.chart_series_style),
                    Message::ChartSeriesStyleChanged,
                )
                .padding([4, 8])
                .text_size(12)
                .width(Length::Fixed(170.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                column![
                    text("Hollow candles").size(12).color(theme.palette().text),
                    text(self.chart_hollow_candle_mode.label())
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(2)
                .width(Fill),
                pick_list(
                    ChartHollowCandleMode::ALL.to_vec(),
                    Some(self.chart_hollow_candle_mode),
                    Message::ChartHollowCandleModeChanged,
                )
                .padding([4, 8])
                .text_size(12)
                .width(Length::Fixed(170.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8);

        let mut effects_controls = column![
            toggle_status_row(
                &theme,
                "Chart fisheye lens",
                self.chart_fisheye_enabled,
                Message::ToggleChartFisheye,
            ),
            toggle_status_row(
                &theme,
                "Chart chromatic aberration",
                self.chart_chromatic_aberration_enabled,
                Message::ToggleChartChromaticAberration,
            ),
            toggle_status_row(
                &theme,
                "Chart edge blur",
                self.chart_edge_blur_enabled,
                Message::ToggleChartEdgeBlur,
            ),
        ]
        .spacing(8);

        if self.chart_fisheye_enabled {
            effects_controls = effects_controls.push(scale_slider_row(
                &theme,
                "Lens",
                self.chart_fisheye_strength,
                MIN_CHART_FISHEYE_STRENGTH..=MAX_CHART_FISHEYE_STRENGTH,
                Message::ChartFisheyeStrengthChanged,
            ));
        }

        if self.chart_chromatic_aberration_enabled {
            effects_controls = effects_controls.push(scale_slider_row(
                &theme,
                "Fringe",
                self.chart_chromatic_aberration_strength,
                MIN_CHART_CHROMATIC_ABERRATION_STRENGTH..=MAX_CHART_CHROMATIC_ABERRATION_STRENGTH,
                Message::ChartChromaticAberrationStrengthChanged,
            ));
        }

        if self.chart_edge_blur_enabled {
            effects_controls = effects_controls.push(scale_slider_row(
                &theme,
                "Blur",
                self.chart_edge_blur_strength,
                MIN_CHART_EDGE_BLUR_STRENGTH..=MAX_CHART_EDGE_BLUR_STRENGTH,
                Message::ChartEdgeBlurStrengthChanged,
            ));
        }

        column![
            appearance_preview(
                &theme,
                AppearancePreview {
                    ui_scale: self.ui_scale,
                    pane_border_thickness: self.pane_border_thickness,
                    pane_corner_radius: self.pane_corner_radius,
                    widget_padding: self.widget_padding_default,
                    outer_widget_border_enabled: self.outer_widget_border_enabled,
                    custom_window_chrome_enabled: self.custom_window_chrome_enabled,
                    chart_dotted_background: self.chart_dotted_background,
                    chart_dotted_background_opacity: self.chart_dotted_background_opacity,
                    chart_gradient_background: self.chart_gradient_background,
                    chart_series_style: self.chart_series_style,
                    chart_hollow_candle_mode: self.chart_hollow_candle_mode,
                    chart_fisheye_enabled: self.chart_fisheye_enabled,
                    chart_chromatic_aberration_enabled: self.chart_chromatic_aberration_enabled,
                    chart_edge_blur_enabled: self.chart_edge_blur_enabled,
                },
            ),
            appearance_section(&theme, "Interface", interface_controls.into()),
            appearance_section(&theme, "Window Chrome", window_controls.into()),
            appearance_section(&theme, "Chart Background", background_controls.into()),
            appearance_section(&theme, "Chart Style", chart_style_controls.into()),
            appearance_section(&theme, "Chart Effects", effects_controls.into()),
            default_settings_reference(&theme),
        ]
        .spacing(16)
        .into()
    }

    fn view_focused_widget_padding_controls(&self, theme: &Theme) -> Element<'_, Message> {
        let Some((pane, target)) = self.focused_widget_padding_target() else {
            return text("Selected widget: none")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into();
        };
        let Some(kind) = self.panes.get(pane) else {
            return text("Selected widget: none")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into();
        };

        let title = pane_title(kind);
        let padding = self.widget_padding_for_target(&target);
        let uses_override = self.widget_padding_overrides.contains_key(&target);
        let reset = if uses_override {
            button(text("Use default").size(12))
                .on_press(Message::ResetFocusedWidgetPadding)
                .padding([4, 8])
        } else {
            button(text("Use default").size(12)).padding([4, 8])
        };

        column![
            row![
                text(format!("Selected: {title}"))
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .width(Fill),
                reset,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            chrome_slider_row(
                theme,
                "Selected",
                padding,
                MIN_WIDGET_PADDING..=MAX_WIDGET_PADDING,
                Message::FocusedWidgetPaddingChanged,
            ),
        ]
        .spacing(6)
        .into()
    }

    pub(super) fn view_settings_crosshair_section(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let game_hud_selected = self.chart_crosshair_style.is_game_hud();
        let defaults = if game_hud_selected {
            format!(
                "Defaults: {} style, full-span guides on, {:.0}% size, {} HUD sound, {:.0}% HUD volume",
                ChartCrosshairStyle::default().label(),
                DEFAULT_CHART_CROSSHAIR_SCALE * 100.0,
                ChartHudOrderSound::default().label(),
                DEFAULT_CHART_HUD_ORDER_SOUND_VOLUME * 100.0,
            )
        } else {
            format!(
                "Defaults: {} style, full-span guides on, {:.0}% size",
                ChartCrosshairStyle::default().label(),
                DEFAULT_CHART_CROSSHAIR_SCALE * 100.0,
            )
        };

        let mut content = Column::new()
            .push(
                checkbox(self.chart_crosshair_guides_enabled)
                    .label("Full-span guide lines")
                    .on_toggle(Message::ToggleChartCrosshairGuides)
                    .size(12)
                    .spacing(8)
                    .text_size(12)
                    .font(crate::app_fonts::monospace_font()),
            )
            .push(scale_slider_row(
                &theme,
                "Size",
                self.chart_crosshair_scale,
                MIN_CHART_CROSSHAIR_SCALE..=MAX_CHART_CROSSHAIR_SCALE,
                Message::ChartCrosshairScaleChanged,
            ))
            .push(crosshair_style_grid(
                &theme,
                self.chart_crosshair_style,
                self.chart_crosshair_guides_enabled,
                self.chart_crosshair_scale,
            ));

        if game_hud_selected {
            content = content
                .push(gaming_hud_style_grid(
                    &theme,
                    self.chart_crosshair_style,
                    self.chart_crosshair_guides_enabled,
                    self.chart_crosshair_scale,
                ))
                .push(hud_readout_settings(&theme, self.chart_hud_readout))
                .push(hud_order_sound_settings(
                    &theme,
                    self.chart_hud_order_sound,
                    self.chart_hud_order_sound_file.as_deref(),
                    self.chart_hud_order_sound_volume,
                ))
                .push(
                    checkbox(self.chart_hud_ui_sounds)
                        .label("HUD control clicks (mode, side, arm, size)")
                        .on_toggle(Message::ToggleChartHudUiSounds)
                        .size(12)
                        .spacing(8)
                        .text_size(12)
                        .font(crate::app_fonts::monospace_font()),
                );
        }

        content
            .push(
                text(defaults)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            )
            .spacing(10)
            .into()
    }
}

#[derive(Debug, Clone, Copy)]
struct AppearancePreview {
    ui_scale: f32,
    pane_border_thickness: f32,
    pane_corner_radius: f32,
    widget_padding: f32,
    outer_widget_border_enabled: bool,
    custom_window_chrome_enabled: bool,
    chart_dotted_background: bool,
    chart_dotted_background_opacity: f32,
    chart_gradient_background: bool,
    chart_series_style: ChartSeriesStyle,
    chart_hollow_candle_mode: ChartHollowCandleMode,
    chart_fisheye_enabled: bool,
    chart_chromatic_aberration_enabled: bool,
    chart_edge_blur_enabled: bool,
}

fn appearance_preview(theme: &Theme, preview: AppearancePreview) -> Element<'static, Message> {
    column![
        iced::widget::canvas(preview)
            .width(Fill)
            .height(Length::Fixed(132.0)),
        row![
            summary_chip(theme, "Scale", format!("{:.0}%", preview.ui_scale * 100.0)),
            summary_chip(
                theme,
                "Divider",
                format!("{:.0}px", preview.pane_border_thickness)
            ),
            summary_chip(
                theme,
                "Corners",
                format!("{:.0}px", preview.pane_corner_radius)
            ),
        ]
        .spacing(6),
        row![
            Space::new().width(Fill),
            summary_chip(theme, "Padding", format!("{:.0}px", preview.widget_padding)),
            summary_chip(
                theme,
                "Series",
                preview.chart_series_style.label().to_string()
            ),
        ]
        .spacing(6),
    ]
    .spacing(8)
    .into()
}

fn appearance_section<'a>(
    theme: &Theme,
    title: &'static str,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        text(title).size(13).color(theme.palette().text).width(Fill),
        rule::horizontal(1),
        content,
    ]
    .spacing(8)
    .into()
}

fn toggle_status_row(
    theme: &Theme,
    label: &'static str,
    enabled: bool,
    on_toggle: fn(bool) -> Message,
) -> Element<'static, Message> {
    let status_color = if enabled {
        theme.palette().success
    } else {
        theme.extended_palette().background.weak.text
    };

    row![
        checkbox(enabled)
            .label(label)
            .on_toggle(on_toggle)
            .size(12)
            .spacing(8)
            .text_size(12)
            .font(crate::app_fonts::monospace_font()),
        Space::new().width(Fill),
        text(if enabled { "On" } else { "Off" })
            .size(11)
            .color(status_color)
            .width(Length::Fixed(34.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn default_settings_reference(theme: &Theme) -> Element<'static, Message> {
    column![
        row![
            text("Defaults")
                .size(13)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill),
            default_chip(theme, format!("{:.0}% scale", DEFAULT_UI_SCALE * 100.0)),
            default_chip(
                theme,
                format!("{:.0}px divider", default_pane_border_thickness())
            ),
            default_chip(
                theme,
                format!("{:.0}px corners", default_pane_corner_radius())
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        row![
            Space::new().width(Fill),
            default_chip(theme, format!("{:.0}px padding", default_widget_padding())),
            default_chip(
                theme,
                format!(
                    "{:.0}% dots",
                    DEFAULT_CHART_DOTTED_BACKGROUND_OPACITY * 100.0
                )
            ),
            default_chip(
                theme,
                format!("{:.0}% lens", DEFAULT_CHART_FISHEYE_STRENGTH * 100.0)
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        row![
            Space::new().width(Fill),
            default_chip(
                theme,
                format!(
                    "{:.0}% fringe",
                    DEFAULT_CHART_CHROMATIC_ABERRATION_STRENGTH * 100.0
                )
            ),
            default_chip(
                theme,
                format!("{:.0}% blur", DEFAULT_CHART_EDGE_BLUR_STRENGTH * 100.0)
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(6)
    .into()
}

fn summary_chip(theme: &Theme, label: &'static str, value: String) -> Element<'static, Message> {
    let text_color = theme.palette().text;
    let muted = theme.extended_palette().background.weak.text;

    container(
        row![
            text(label).size(10).color(muted),
            text(value).size(10).color(text_color),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([4, 7])
    .style(chip_container_style)
    .into()
}

fn default_chip(theme: &Theme, value: String) -> Element<'static, Message> {
    container(
        text(value)
            .size(10)
            .color(theme.extended_palette().background.weak.text),
    )
    .padding([4, 7])
    .style(chip_container_style)
    .into()
}

fn chip_container_style(theme: &Theme) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.weak.text;
    border_color.a = 0.18;

    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    }
}

impl iced::widget::canvas::Program<Message> for AppearancePreview {
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

        let bg = theme.extended_palette().background.base.color;
        let weak = theme.extended_palette().background.weak.color;
        let strong = theme.extended_palette().background.strong.color;
        let muted = theme.extended_palette().background.weak.text;
        let primary = theme.palette().primary;
        let success = theme.palette().success;
        let danger = theme.palette().danger;

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), bg);

        let margin = 9.0;
        let chrome_h = if self.custom_window_chrome_enabled {
            16.0
        } else {
            6.0
        };
        let x = margin;
        let y = margin;
        let w = (bounds.width - margin * 2.0).max(20.0);
        let h = (bounds.height - margin * 2.0).max(20.0);
        let outer = iced::widget::canvas::Path::rectangle(Point::new(x, y), Size::new(w, h));
        frame.fill(&outer, strong);
        if self.outer_widget_border_enabled {
            frame.stroke(
                &outer,
                iced::widget::canvas::Stroke::default()
                    .with_color(Color { a: 0.42, ..primary })
                    .with_width(1.0),
            );
        }

        if self.custom_window_chrome_enabled {
            frame.fill_rectangle(Point::new(x, y), Size::new(w, chrome_h), weak);
            for i in 0..3 {
                frame.fill_rectangle(
                    Point::new(x + 8.0 + i as f32 * 9.0, y + 5.0),
                    Size::new(4.0, 4.0),
                    Color { a: 0.75, ..muted },
                );
            }
        }

        let content_y = y + chrome_h;
        let content_h = (h - chrome_h).max(8.0);
        let divider = self.pane_border_thickness.clamp(1.0, 8.0);
        let left_w = w * 0.34;
        let chart_x = x + left_w + divider;
        let chart_w = (w - left_w - divider).max(8.0);
        frame.fill_rectangle(Point::new(x, content_y), Size::new(left_w, content_h), bg);
        frame.fill_rectangle(
            Point::new(x + left_w, content_y),
            Size::new(divider, content_h),
            muted,
        );

        let pad = self.widget_padding.clamp(2.0, 20.0);
        let row_h = ((content_h - pad * 2.0 - 10.0) / 4.0).max(4.0);
        for i in 0..4 {
            let row_y = content_y + pad + i as f32 * (row_h + 3.0);
            let row_w = (left_w - pad * 2.0 - i as f32 * 4.0).max(4.0);
            frame.fill_rectangle(
                Point::new(x + pad, row_y),
                Size::new(row_w, row_h),
                Color {
                    a: if i == 0 { 0.50 } else { 0.26 },
                    ..weak
                },
            );
        }

        let chart_rect = iced::widget::canvas::Path::rectangle(
            Point::new(chart_x, content_y),
            Size::new(chart_w, content_h),
        );
        frame.fill(&chart_rect, bg);
        if self.chart_gradient_background {
            frame.fill_rectangle(
                Point::new(chart_x, content_y),
                Size::new(chart_w, content_h * 0.55),
                Color { a: 0.22, ..primary },
            );
        }
        if self.chart_dotted_background {
            let dot_alpha = self.chart_dotted_background_opacity.clamp(0.0, 1.0) * 0.55;
            let mut dot_y = content_y + 9.0;
            while dot_y < content_y + content_h - 4.0 {
                let mut dot_x = chart_x + 9.0;
                while dot_x < chart_x + chart_w - 4.0 {
                    frame.fill_rectangle(
                        Point::new(dot_x, dot_y),
                        Size::new(1.6, 1.6),
                        Color {
                            a: dot_alpha,
                            ..muted
                        },
                    );
                    dot_x += 15.0;
                }
                dot_y += 15.0;
            }
        }

        if self.chart_series_style.is_line() {
            let line = iced::widget::canvas::Path::new(|p| {
                p.move_to(Point::new(chart_x + 9.0, content_y + content_h * 0.70));
                p.line_to(Point::new(
                    chart_x + chart_w * 0.28,
                    content_y + content_h * 0.45,
                ));
                p.line_to(Point::new(
                    chart_x + chart_w * 0.48,
                    content_y + content_h * 0.57,
                ));
                p.line_to(Point::new(
                    chart_x + chart_w * 0.68,
                    content_y + content_h * 0.31,
                ));
                p.line_to(Point::new(
                    chart_x + chart_w - 9.0,
                    content_y + content_h * 0.39,
                ));
            });
            if self.chart_chromatic_aberration_enabled {
                let fringe = iced::widget::canvas::Path::new(|p| {
                    p.move_to(Point::new(chart_x + 11.0, content_y + content_h * 0.71));
                    p.line_to(Point::new(
                        chart_x + chart_w * 0.28 + 2.0,
                        content_y + content_h * 0.46,
                    ));
                    p.line_to(Point::new(
                        chart_x + chart_w * 0.48 + 2.0,
                        content_y + content_h * 0.58,
                    ));
                    p.line_to(Point::new(
                        chart_x + chart_w * 0.68 + 2.0,
                        content_y + content_h * 0.32,
                    ));
                    p.line_to(Point::new(
                        chart_x + chart_w - 7.0,
                        content_y + content_h * 0.40,
                    ));
                });
                frame.stroke(
                    &fringe,
                    iced::widget::canvas::Stroke::default()
                        .with_color(Color { a: 0.5, ..danger })
                        .with_width(1.0),
                );
            }
            frame.stroke(
                &line,
                iced::widget::canvas::Stroke::default()
                    .with_color(primary)
                    .with_width(2.0),
            );
        } else {
            for i in 0..6 {
                let candle_x = chart_x + 12.0 + i as f32 * ((chart_w - 24.0) / 6.0);
                let bullish = i % 2 == 0;
                let color = if bullish { success } else { danger };
                let body_h = 16.0 + (i % 3) as f32 * 4.0;
                let body_y = content_y + content_h * 0.35 + (i % 2) as f32 * 8.0;
                let wick_x = candle_x + 4.0;
                let hollow = self.chart_hollow_candle_mode.applies_to(bullish);
                let wick = iced::widget::canvas::Path::line(
                    Point::new(wick_x, body_y - 8.0),
                    Point::new(wick_x, body_y + body_h + 8.0),
                );
                frame.stroke(
                    &wick,
                    iced::widget::canvas::Stroke::default()
                        .with_color(color)
                        .with_width(1.0),
                );
                let body = iced::widget::canvas::Path::rectangle(
                    Point::new(candle_x, body_y),
                    Size::new(8.0, body_h),
                );
                if hollow {
                    frame.stroke(
                        &body,
                        iced::widget::canvas::Stroke::default()
                            .with_color(color)
                            .with_width(1.2),
                    );
                } else {
                    frame.fill(&body, color);
                }
            }
        }

        if self.chart_fisheye_enabled {
            let lens = iced::widget::canvas::Path::rectangle(
                Point::new(chart_x + chart_w * 0.43, content_y + content_h * 0.24),
                Size::new(chart_w * 0.22, content_h * 0.42),
            );
            frame.stroke(
                &lens,
                iced::widget::canvas::Stroke::default()
                    .with_color(Color { a: 0.36, ..primary })
                    .with_width(1.0),
            );
        }

        if self.chart_edge_blur_enabled {
            frame.fill_rectangle(
                Point::new(chart_x, content_y),
                Size::new(chart_w, 8.0),
                Color { a: 0.20, ..strong },
            );
            frame.fill_rectangle(
                Point::new(chart_x, content_y + content_h - 8.0),
                Size::new(chart_w, 8.0),
                Color { a: 0.20, ..strong },
            );
        }

        if self.pane_corner_radius > 0.0 {
            let corner = self.pane_corner_radius.min(10.0);
            for (cx, cy) in [
                (x + 2.0, y + 2.0),
                (x + w - corner - 2.0, y + 2.0),
                (x + 2.0, y + h - corner - 2.0),
                (x + w - corner - 2.0, y + h - corner - 2.0),
            ] {
                frame.fill_rectangle(
                    Point::new(cx, cy),
                    Size::new(corner, corner),
                    Color { a: 0.16, ..primary },
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn hud_readout_settings(theme: &Theme, config: ChartHudReadoutConfig) -> Element<'static, Message> {
    let mut toggles = Column::new()
        .push(text("HUD readout").size(13).color(theme.palette().text))
        .spacing(7);

    for element in ChartHudReadoutElement::ALL {
        toggles = toggles.push(hud_readout_toggle(config, element));
    }

    toggles.into()
}

fn hud_readout_toggle(
    config: ChartHudReadoutConfig,
    element: ChartHudReadoutElement,
) -> Element<'static, Message> {
    checkbox(config.enabled(element))
        .label(element.label())
        .on_toggle(move |enabled| Message::ChartHudReadoutToggled(element, enabled))
        .size(12)
        .spacing(8)
        .text_size(12)
        .font(crate::app_fonts::monospace_font())
        .into()
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
    let mut entries = Vec::with_capacity(ChartCrosshairStyle::CROSSHAIRS.len() + 1);
    entries.extend(
        ChartCrosshairStyle::CROSSHAIRS
            .iter()
            .take(4)
            .copied()
            .map(CrosshairPickerEntry::Style),
    );
    entries.push(CrosshairPickerEntry::GamingHud);
    entries.extend(
        ChartCrosshairStyle::CROSSHAIRS
            .iter()
            .skip(4)
            .copied()
            .map(CrosshairPickerEntry::Style),
    );

    for entries in entries.chunks(2) {
        let mut style_row = Row::new().spacing(6).width(Fill);
        for entry in entries {
            style_row = match entry {
                CrosshairPickerEntry::Style(style) => style_row.push(crosshair_style_card(
                    theme,
                    *style,
                    selected,
                    guide_lines_enabled,
                    crosshair_scale,
                )),
                CrosshairPickerEntry::GamingHud => style_row.push(gaming_hud_picker_card(
                    theme,
                    selected,
                    guide_lines_enabled,
                    crosshair_scale,
                )),
            };
        }
        grid = grid.push(style_row);
    }

    grid.into()
}

#[derive(Debug, Clone, Copy)]
enum CrosshairPickerEntry {
    Style(ChartCrosshairStyle),
    GamingHud,
}

fn gaming_hud_style_grid(
    theme: &Theme,
    selected: ChartCrosshairStyle,
    guide_lines_enabled: bool,
    crosshair_scale: f32,
) -> Element<'static, Message> {
    let mut row = Row::new().spacing(6).width(Fill);
    for style in ChartCrosshairStyle::GAME_HUDS {
        row = row.push(crosshair_style_card(
            theme,
            style,
            selected,
            guide_lines_enabled,
            crosshair_scale,
        ));
    }

    Column::new()
        .push(text("Gaming HUD").size(13).color(theme.palette().text))
        .push(row)
        .spacing(7)
        .into()
}

fn gaming_hud_picker_card(
    theme: &Theme,
    selected: ChartCrosshairStyle,
    guide_lines_enabled: bool,
    crosshair_scale: f32,
) -> Element<'static, Message> {
    let preview_style = if selected.is_game_hud() {
        selected
    } else {
        ChartCrosshairStyle::Hud
    };
    let on_press_style = if selected.is_game_hud() {
        selected
    } else {
        ChartCrosshairStyle::Hud
    };

    crosshair_style_button(
        theme,
        preview_style,
        "Gaming HUD",
        selected.is_game_hud(),
        guide_lines_enabled,
        crosshair_scale,
        on_press_style,
    )
}

fn crosshair_style_card(
    theme: &Theme,
    style: ChartCrosshairStyle,
    selected: ChartCrosshairStyle,
    guide_lines_enabled: bool,
    crosshair_scale: f32,
) -> Element<'static, Message> {
    crosshair_style_button(
        theme,
        style,
        style.label(),
        style == selected,
        guide_lines_enabled,
        crosshair_scale,
        style,
    )
}

fn crosshair_style_button(
    theme: &Theme,
    preview_style: ChartCrosshairStyle,
    label: &'static str,
    is_selected: bool,
    guide_lines_enabled: bool,
    crosshair_scale: f32,
    on_press_style: ChartCrosshairStyle,
) -> Element<'static, Message> {
    let label_color = if is_selected {
        theme.palette().primary
    } else {
        theme.extended_palette().background.weak.text
    };

    let preview: Element<'static, Message> = iced::widget::canvas(CrosshairStylePreview {
        style: preview_style,
        guide_lines_enabled,
        crosshair_scale,
    })
    .width(Fill)
    .height(Length::Fixed(38.0))
    .into();

    let content = column![
        preview,
        text(label)
            .size(10)
            .color(label_color)
            .font(crate::app_fonts::monospace_font())
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .width(Fill);

    button(content)
        .on_press(Message::ChartCrosshairStyleChanged(on_press_style))
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
                racing_hud_metrics: (self.style == ChartCrosshairStyle::RacingHud)
                    .then(RacingHudMetrics::preview),
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
