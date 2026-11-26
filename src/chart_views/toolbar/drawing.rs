use super::sections::compact_toolbar_button_style;
use crate::annotations::{DrawingMenu, DrawingTool};
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::helpers::selected_row_button_style;
use crate::message::Message;
use iced::widget::{Space, button, column, container, opaque, row, rule, text};
use iced::{Alignment, Element, Fill};

fn tool_label(tool: DrawingTool) -> (&'static str, &'static str) {
    match tool {
        DrawingTool::Select => ("↖", "Select"),
        DrawingTool::Pen => ("✎", "Pen"),
        DrawingTool::HorizontalLevel => ("—", "Horizontal level"),
        DrawingTool::VerticalLine => ("│", "Vertical line"),
        DrawingTool::TrendLine => ("╱", "Trend line"),
        DrawingTool::Ray => ("↗", "Ray"),
        DrawingTool::ExtendedLine => ("↔", "Extended line"),
        DrawingTool::Rectangle => ("▭", "Rectangle"),
        DrawingTool::Measure => ("Δ", "Measure"),
        DrawingTool::FibRetracement => ("≡", "Fib retracement"),
        DrawingTool::FibExtension => ("≣", "Fib extension"),
        DrawingTool::Eraser => ("✗", "Eraser"),
    }
}

pub(super) fn view_drawing_button(
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    instance: &ChartInstance,
    active_tool: Option<DrawingTool>,
) -> Element<'static, Message> {
    let open = instance.drawing_menu == Some((surface_id, DrawingMenu::Tools));
    let (glyph, name) = active_tool.map(tool_label).unwrap_or(("✎", "Draw"));
    let short_name = match active_tool {
        Some(DrawingTool::HorizontalLevel) => "H level",
        Some(DrawingTool::VerticalLine) => "V line",
        Some(DrawingTool::ExtendedLine) => "Extended",
        Some(DrawingTool::FibRetracement) => "Fib retrace",
        Some(DrawingTool::FibExtension) => "Fib extend",
        _ => name,
    };
    button(
        row![
            text(glyph).size(12),
            text(short_name).size(11),
            text("▾").size(10)
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .on_press(Message::ToggleDrawingMenu(
        chart_id,
        surface_id,
        DrawingMenu::Tools,
    ))
    .style(move |theme, status| {
        compact_toolbar_button_style(theme, status, active_tool.is_some() || open)
    })
    .into()
}

pub(super) fn tool_palette(
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    active_tool: Option<DrawingTool>,
) -> Element<'static, Message> {
    use DrawingTool::*;
    let option = |tool: Option<DrawingTool>| {
        let (glyph, name) = tool.map(tool_label).unwrap_or(("✥", "Pan"));
        let selected = active_tool == tool;
        button(
            row![text(glyph).size(13).width(16).center(), text(name).size(12)]
                .spacing(5)
                .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([4, 5])
        .on_press(Message::SetDrawingTool(chart_id, surface_id, tool))
        .style(move |theme, status| selected_row_button_style(theme, status, selected))
    };
    let pair = |a, b| row![option(a), option(Some(b))].spacing(2);
    let tools = column![
        pair(None, Select),
        pair(Some(Pen), Eraser),
        rule::horizontal(1),
        pair(Some(HorizontalLevel), VerticalLine),
        pair(Some(TrendLine), Ray),
        row![option(Some(ExtendedLine)), Space::new().width(Fill)].spacing(2),
        rule::horizontal(1),
        pair(Some(Rectangle), Measure),
        pair(Some(FibRetracement), FibExtension),
    ]
    .spacing(2);
    opaque(
        container(tools)
            .padding(4)
            .width(284)
            .style(container::bordered_box),
    )
}
