use super::sections::chart_toolbar_button;
use crate::account_views::{AnchoredAccountMenu, AnchoredMenuLayer, MenuAlignment, MenuKind};
use crate::annotations::{
    AnnotationId, AnnotationStyle, DEFAULT_LEVEL_COLOR, DEFAULT_LINE_COLOR, DEFAULT_MEASURE_COLOR,
    DrawingMenu, DrawingTool, LineStyle,
};
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::message::Message;
use iced::widget::{Row, Space, button, container, opaque, row, text, tooltip};
use iced::{Alignment, Color, Element, Fill, Padding, Theme};

pub(in crate::chart_views) fn view_annotation_style_bar(
    chart_id: ChartId,
    surface_id: ChartSurfaceId,
    instance: &ChartInstance,
    active_tool: Option<DrawingTool>,
) -> Option<Element<'static, Message>> {
    if active_tool != Some(DrawingTool::Select) {
        return None;
    }
    let id = instance.selected_annotation?;
    let style = &instance.annotations.iter().find(|ann| ann.id == id)?.style;
    let colors = DrawingMenu::Colors(id);
    let trigger = tooltip(
        color_button(
            style.color,
            Message::ToggleDrawingMenu(chart_id, surface_id, colors),
        ),
        text("Drawing color").size(10),
        tooltip::Position::Top,
    );
    let menu = (instance.drawing_menu == Some((surface_id, colors))).then(|| AnchoredMenuLayer {
        kind: MenuKind::DrawingColors,
        alignment: MenuAlignment::Start,
        content: color_palette(chart_id, id, style),
    });
    let controls = row![
        AnchoredAccountMenu::new(trigger.into(), menu),
        annotation_line_style_button(chart_id, id, style),
        annotation_width_button(chart_id, id, style),
        annotation_lock_button(chart_id, id, style),
        button(text("Delete").size(11))
            .padding([3, 6])
            .on_press_maybe((!style.locked).then_some(Message::RemoveAnnotation(chart_id, id)))
            .style(button::text),
    ]
    .spacing(2)
    .align_y(Alignment::Center)
    .wrap();
    let strip = opaque(
        container(controls)
            .padding(3)
            .style(container::bordered_box),
    );
    Some(
        container(strip)
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::End)
            .padding(Padding::from(6).bottom(crate::chart::TIME_AXIS_HEIGHT + 4.0))
            .into(),
    )
}

fn color_palette(
    chart_id: ChartId,
    id: AnnotationId,
    style: &AnnotationStyle,
) -> Element<'static, Message> {
    let mut swatches = Row::new().spacing(3);
    for color in [
        DEFAULT_LINE_COLOR,
        DEFAULT_LEVEL_COLOR,
        DEFAULT_MEASURE_COLOR,
        Color::from_rgb(0.95, 0.45, 0.45),
        Color::from_rgb(0.62, 0.55, 0.95),
        Color::from_rgb(0.92, 0.92, 0.92),
    ] {
        let next = restyle(style, |s| {
            s.color = Color {
                a: s.color.a,
                ..color
            }
        });
        swatches = swatches.push(color_button(
            color,
            Message::RestyleAnnotation(chart_id, id, next),
        ));
    }
    opaque(
        container(swatches)
            .padding(4)
            .style(container::bordered_box),
    )
}

fn color_button(color: Color, message: Message) -> Element<'static, Message> {
    button(Space::new())
        .width(20)
        .height(20)
        .on_press(message)
        .style(move |theme: &Theme, status| button::Style {
            background: Some(color.into()),
            border: iced::Border {
                color: if status == button::Status::Hovered {
                    theme.palette().text
                } else {
                    theme.extended_palette().background.strong.color
                },
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

const ANNOTATION_WIDTH_STEPS: [f32; 4] = [1.0, 1.5, 2.5, 4.0];

fn restyle(style: &AnnotationStyle, mutate: impl FnOnce(&mut AnnotationStyle)) -> AnnotationStyle {
    let mut next = style.clone();
    mutate(&mut next);
    next
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
    let label = format!("{}px", style.width);
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
    let label = if style.locked { "Unlock" } else { "Lock" };
    tooltip(
        chart_toolbar_button(
            label,
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
