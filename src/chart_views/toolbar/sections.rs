use crate::annotations::{
    AnnotationId, AnnotationStyle, DEFAULT_LEVEL_COLOR, DEFAULT_LINE_COLOR, DEFAULT_MEASURE_COLOR,
    DrawingTool, LineStyle,
};
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::message::Message;
use iced::widget::{Row, button, column, container, rule, text, tooltip};
use iced::{Color, Element, Fill, Length, Theme};

pub(super) fn chart_toolbar_strip<'a>(
    controls: Row<'a, Message>,
    tools: Row<'a, Message>,
) -> Element<'a, Message> {
    let body = column![
        controls.width(Fill).wrap().vertical_spacing(0),
        drawing_toolbar_divider(),
        tools.width(Fill).wrap().vertical_spacing(0),
    ]
    .spacing(2)
    .width(Fill);

    container(body)
        .width(Fill)
        .style(|theme: &Theme| {
            let background = Color {
                a: 0.04,
                ..theme.extended_palette().background.weak.color
            };
            container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        })
        .into()
}

/// Faint divider separating the controls row from the drawing-tools row.
fn drawing_toolbar_divider() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.08,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

/// The second toolbar row: a collapse toggle followed by the drawing tools and
/// (when something is selected) the style bar. Collapsed, only the toggle shows.
pub(super) fn view_drawing_toolbar_row<'a>(
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    instance: &ChartInstance,
    active_tool: Option<DrawingTool>,
) -> Row<'a, Message> {
    let collapsed = instance.drawing_toolbar_collapsed;
    let mut row = Row::new()
        .spacing(0)
        .align_y(iced::Alignment::Center)
        .push(drawing_toolbar_toggle(chart_id, collapsed));

    if !collapsed {
        row = push_drawing_tool_buttons(row, chart_id, surface_id, active_tool);
        row = push_annotation_style_bar(row, chart_id, instance, active_tool);
    }
    row
}

fn drawing_toolbar_toggle(chart_id: ChartId, collapsed: bool) -> Element<'static, Message> {
    let (label, tip) = if collapsed {
        ("\u{270E} \u{25B8} Draw", "Show drawing tools")
    } else {
        ("\u{270E} \u{25BE} Draw", "Hide drawing tools")
    };
    tooltip(
        chart_toolbar_button(
            label,
            !collapsed,
            Message::ToggleChartDrawingToolbar(chart_id),
        ),
        text(tip).size(10).font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

pub(super) fn chart_toolbar_button(
    label: &'static str,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label).size(11).center())
        .on_press(msg)
        .padding([3, 8])
        .style(move |theme: &Theme, status| compact_toolbar_button_style(theme, status, active))
        .into()
}

pub(super) fn chart_reload_button(chart_id: ChartId) -> Element<'static, Message> {
    button(text("\u{27F3}").size(12))
        .on_press(Message::ChartReload(chart_id))
        .padding([3, 8])
        .style(|theme: &Theme, status| compact_toolbar_button_style(theme, status, false))
        .into()
}

pub(super) fn chart_reset_view_button(
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
) -> Element<'static, Message> {
    chart_toolbar_button(
        "Reset View",
        false,
        Message::ChartResetView(chart_id, surface_id),
    )
}

pub(super) fn chart_fetch_status_label(
    has_candles: bool,
    instance: &ChartInstance,
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    if has_candles && instance.candle_fetch_request.is_some() {
        Some(chart_toolbar_status_label(
            "Refreshing",
            theme.palette().warning,
        ))
    } else if has_candles && instance.candle_fetch_error.is_some() {
        Some(chart_toolbar_status_label("Stale", theme.palette().danger))
    } else if instance.secondary_candle_fetch_request.is_some() {
        Some(chart_toolbar_status_label(
            "CMP Refreshing",
            theme.palette().warning,
        ))
    } else if instance.secondary_candle_fetch_error.is_some() {
        Some(chart_toolbar_status_label(
            "CMP Stale",
            theme.palette().danger,
        ))
    } else {
        None
    }
}

/// (glyph, tooltip, tool) for each drawing tool button, in toolbar order.
const DRAWING_TOOLS: &[(&str, &str, DrawingTool)] = &[
    (
        "\u{2196}",
        "Select / move / edit drawings",
        DrawingTool::Select,
    ),
    ("\u{2014}", "Horizontal level", DrawingTool::HorizontalLevel),
    ("\u{2502}", "Vertical line", DrawingTool::VerticalLine),
    ("\u{2571}", "Trend line", DrawingTool::TrendLine),
    ("\u{2197}", "Ray (extends one way)", DrawingTool::Ray),
    (
        "\u{2194}",
        "Extended line (extends both ways)",
        DrawingTool::ExtendedLine,
    ),
    ("\u{25AD}", "Rectangle / zone", DrawingTool::Rectangle),
    ("\u{0394}", "Measure price / time", DrawingTool::Measure),
    (
        "\u{2261}",
        "Fibonacci retracement",
        DrawingTool::FibRetracement,
    ),
    ("\u{2263}", "Fibonacci extension", DrawingTool::FibExtension),
    (
        "\u{2717}",
        "Eraser (click a drawing to delete)",
        DrawingTool::Eraser,
    ),
];

pub(super) fn push_drawing_tool_buttons<'a>(
    mut toolbar: Row<'a, Message>,
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    active_tool: Option<DrawingTool>,
) -> Row<'a, Message> {
    for (glyph, tip, tool) in DRAWING_TOOLS {
        toolbar = toolbar.push(chart_toolbar_separator()).push(tooltip(
            drawing_tool_button(glyph, chart_id, surface_id, active_tool, *tool),
            text(*tip).size(10).font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ));
    }
    toolbar
}

/// Preset colors offered in the annotation style bar.
fn annotation_palette() -> [Color; 6] {
    [
        DEFAULT_LINE_COLOR,
        DEFAULT_LEVEL_COLOR,
        DEFAULT_MEASURE_COLOR,
        Color::from_rgb(0.95, 0.45, 0.45),
        Color::from_rgb(0.62, 0.55, 0.95),
        Color::from_rgb(0.92, 0.92, 0.92),
    ]
}

const ANNOTATION_WIDTH_STEPS: [f32; 4] = [1.0, 1.5, 2.5, 4.0];

/// Append a compact style bar for the currently selected annotation when the
/// Select tool is active. Recolor / restyle / lock / delete the selection.
pub(super) fn push_annotation_style_bar<'a>(
    toolbar: Row<'a, Message>,
    chart_id: ChartId,
    instance: &ChartInstance,
    active_tool: Option<DrawingTool>,
) -> Row<'a, Message> {
    if active_tool != Some(DrawingTool::Select) {
        return toolbar;
    }
    let Some(id) = instance.selected_annotation else {
        return toolbar;
    };
    let Some(annotation) = instance.annotations.iter().find(|ann| ann.id == id) else {
        return toolbar;
    };
    let style = annotation.style.clone();

    let mut bar = toolbar.push(chart_toolbar_separator());
    for color in annotation_palette() {
        bar = bar.push(annotation_color_swatch(chart_id, id, &style, color));
    }

    bar = bar
        .push(chart_toolbar_separator())
        .push(annotation_line_style_button(chart_id, id, &style))
        .push(chart_toolbar_separator())
        .push(annotation_width_button(chart_id, id, &style))
        .push(chart_toolbar_separator())
        .push(annotation_lock_button(chart_id, id, &style))
        .push(chart_toolbar_separator())
        .push(annotation_delete_button(chart_id, id));
    bar
}

fn restyle(style: &AnnotationStyle, mutate: impl FnOnce(&mut AnnotationStyle)) -> AnnotationStyle {
    let mut next = style.clone();
    mutate(&mut next);
    next
}

fn annotation_color_swatch<'a>(
    chart_id: ChartId,
    id: AnnotationId,
    style: &AnnotationStyle,
    color: Color,
) -> Element<'a, Message> {
    let next = restyle(style, |s| {
        s.color = Color {
            a: s.color.a,
            ..color
        };
    });
    button(text(""))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .on_press(Message::RestyleAnnotation(chart_id, id, next))
        .style(move |_theme: &Theme, status| {
            let border_color = match status {
                button::Status::Hovered => Color::WHITE,
                _ => Color {
                    a: 0.4,
                    ..Color::WHITE
                },
            };
            button::Style {
                background: Some(color.into()),
                border: iced::Border {
                    radius: 2.0.into(),
                    width: 1.0,
                    color: border_color,
                },
                ..Default::default()
            }
        })
        .into()
}

fn annotation_line_style_button<'a>(
    chart_id: ChartId,
    id: AnnotationId,
    style: &AnnotationStyle,
) -> Element<'a, Message> {
    let (glyph, next_style) = match style.line_style {
        LineStyle::Solid => ("\u{2015}", LineStyle::Dashed),
        LineStyle::Dashed => ("\u{254C}", LineStyle::Dotted),
        LineStyle::Dotted => ("\u{22EF}", LineStyle::Solid),
    };
    let next = restyle(style, |s| s.line_style = next_style);
    tooltip(
        chart_toolbar_button(glyph, false, Message::RestyleAnnotation(chart_id, id, next)),
        text("Cycle line style")
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

fn annotation_width_button<'a>(
    chart_id: ChartId,
    id: AnnotationId,
    style: &AnnotationStyle,
) -> Element<'a, Message> {
    let next_width = ANNOTATION_WIDTH_STEPS
        .iter()
        .copied()
        .find(|step| *step > style.width + 0.01)
        .unwrap_or(ANNOTATION_WIDTH_STEPS[0]);
    let next = restyle(style, |s| s.width = next_width);
    let label: &'static str = if style.width <= 1.0 {
        "1px"
    } else if style.width <= 1.5 {
        "2px"
    } else if style.width <= 2.5 {
        "3px"
    } else {
        "4px"
    };
    tooltip(
        chart_toolbar_button(label, false, Message::RestyleAnnotation(chart_id, id, next)),
        text("Cycle line width")
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

fn annotation_lock_button<'a>(
    chart_id: ChartId,
    id: AnnotationId,
    style: &AnnotationStyle,
) -> Element<'a, Message> {
    let next = restyle(style, |s| s.locked = !s.locked);
    let glyph = if style.locked {
        "\u{1F512}"
    } else {
        "\u{1F513}"
    };
    tooltip(
        chart_toolbar_button(
            glyph,
            style.locked,
            Message::RestyleAnnotation(chart_id, id, next),
        ),
        text("Lock / unlock drawing")
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

fn annotation_delete_button<'a>(chart_id: ChartId, id: AnnotationId) -> Element<'a, Message> {
    tooltip(
        chart_toolbar_button("\u{2717}", false, Message::RemoveAnnotation(chart_id, id)),
        text("Delete drawing")
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

pub(super) fn push_chart_mode_buttons<'a>(
    toolbar: Row<'a, Message>,
    chart_id: ChartId,
    instance: &ChartInstance,
) -> Row<'a, Message> {
    let mut toolbar = toolbar
        .push(chart_toolbar_separator())
        .push(tooltip(
            chart_toolbar_button(
                "\u{21C5}",
                instance.chart.inverted,
                Message::ToggleChartInvert(chart_id),
            ),
            text("Invert price axis")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ))
        .push(chart_toolbar_separator())
        .push(tooltip(
            chart_toolbar_button(
                "FILL",
                instance.chart.show_trade_markers,
                Message::ToggleChartTradeMarkers(chart_id),
            ),
            text("Show account buy/sell fills")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ));

    toolbar = toolbar.push(chart_toolbar_separator()).push(tooltip(
        chart_toolbar_button(
            "CMP",
            instance.secondary_symbol.is_some() || instance.secondary_editor_open,
            Message::ChartSecondaryOpenEditor(chart_id),
        ),
        text("Add comparison symbol")
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    ));

    if instance.secondary_symbol.is_some() {
        toolbar = toolbar.push(tooltip(
            chart_toolbar_button(
                "Clear CMP",
                false,
                Message::ChartSecondarySymbolRemoved(chart_id),
            ),
            text("Remove comparison symbol")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        ));
    }

    toolbar
}

pub(super) fn chart_toolbar_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.12,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(14)
    .width(1)
    .into()
}

fn drawing_tool_button(
    label: &'static str,
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    active_tool: Option<DrawingTool>,
    tool: DrawingTool,
) -> Element<'static, Message> {
    chart_toolbar_button(
        label,
        active_tool == Some(tool),
        Message::SetDrawingTool(
            chart_id,
            surface_id,
            if active_tool == Some(tool) {
                None
            } else {
                Some(tool)
            },
        ),
    )
}

fn chart_toolbar_status_label(label: &'static str, color: Color) -> Element<'static, Message> {
    container(text(label).size(10).color(color))
        .padding([3, 8])
        .into()
}

fn compact_toolbar_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let background = if active {
        Color {
            a: 0.10,
            ..theme.palette().primary
        }
    } else {
        match status {
            button::Status::Hovered => Color {
                a: 0.55,
                ..theme.extended_palette().background.strong.color
            },
            _ => Color::TRANSPARENT,
        }
    };

    button::Style {
        background: Some(background.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.extended_palette().background.weak.text
        },
        border: iced::Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
