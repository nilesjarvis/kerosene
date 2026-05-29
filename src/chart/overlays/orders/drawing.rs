use super::super::TradingOverlayContext;
use super::segments::{stroke_segmented_hline_range, stroke_segmented_quadratic_curve};
use super::visible::{MOVING_ORDER_LINE_DASH, ORDER_LINE_DASH, VisibleOrder};

use crate::chart::drawing::{AxisBadgeStyle, SegmentedHLineStyle};
use crate::chart::order_labels::{
    ORDER_CANCEL_WIDTH, ORDER_LABEL_CONNECTOR_SPAN, ORDER_LABEL_HEIGHT, ORDER_LABEL_TEXT_X,
    ORDER_LABEL_X, ORDER_PENDING_SPINNER_X, ORDER_PENDING_TEXT_X, OrderLabelPosition,
    order_label_y_range,
};
use crate::chart::price_badges::{
    RIGHT_AXIS_SECONDARY_BADGE_HEIGHT, RightAxisBadgeConnectorStyle, RightAxisBadgeKind,
    draw_stacked_right_axis_badge, right_axis_line_end_x,
};
use crate::helpers::format_price;

use iced::widget::canvas;
use iced::{Color, Point, Radians, Size, alignment};

// ---------------------------------------------------------------------------
// Order Drawing
// ---------------------------------------------------------------------------

pub(super) fn draw_order_line<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    order: &VisibleOrder,
    position: OrderLabelPosition,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let start_x = order_line_start_x(order, position, ctx.chart_w);
    let badge_kind = RightAxisBadgeKind::ActiveOrder(order.order_index);
    let end_x = right_axis_line_end_x(ctx.right_axis_badges, badge_kind, ctx.chart_w);
    let style = order_line_style(order);
    stroke_segmented_hline_range(ctx.frame, ctx.fisheye, start_x, end_x, order.order_y, style);
}

pub(super) fn draw_order_price_badge<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    order: &VisibleOrder,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let badge_kind = RightAxisBadgeKind::ActiveOrder(order.order_index);
    draw_stacked_right_axis_badge(
        ctx.frame,
        ctx.right_axis_badges,
        badge_kind,
        ctx.chart_w,
        order.order_y,
        format_price(order.display_px),
        order.order_color_solid,
        AxisBadgeStyle {
            char_width: 6.5,
            padding_width: 8.0,
            height: RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
            text_size: 9.0,
            text_color: Color::BLACK,
        },
        RightAxisBadgeConnectorStyle::Segmented {
            style: order_line_style(order),
        },
        ctx.fisheye,
    );
}

pub(super) fn draw_order_label_connector<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    order: &VisibleOrder,
    position: OrderLabelPosition,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    if (position.label_y - position.order_y).abs() < 1.0 {
        return;
    }

    let start_x = order_line_start_x(order, position, ctx.chart_w);
    if start_x <= order.label_right_x {
        return;
    }

    let style = order_line_style(order);
    stroke_segmented_quadratic_curve(
        ctx.frame,
        ctx.fisheye,
        Point::new(start_x, order.order_y),
        Point::new(
            order.label_right_x + (start_x - order.label_right_x) * 0.45,
            order.order_y,
        ),
        Point::new(order.label_right_x, position.label_y),
        &style,
    );
}

fn order_line_start_x(order: &VisibleOrder, position: OrderLabelPosition, chart_w: f32) -> f32 {
    if chart_w <= 0.0 || !chart_w.is_finite() {
        return 0.0;
    }

    let start_x = if (position.label_y - position.order_y).abs() < 1.0 {
        order.label_right_x
    } else {
        order.label_right_x + ORDER_LABEL_CONNECTOR_SPAN
    };
    start_x.clamp(0.0, chart_w)
}

fn order_line_style(order: &VisibleOrder) -> SegmentedHLineStyle {
    if order.is_animating {
        SegmentedHLineStyle {
            segment_len: MOVING_ORDER_LINE_DASH[0],
            gap_len: MOVING_ORDER_LINE_DASH[1],
            offset: order.line_offset,
            color: order.order_color,
            width: order.line_width,
        }
    } else {
        SegmentedHLineStyle {
            segment_len: ORDER_LINE_DASH[0],
            gap_len: ORDER_LINE_DASH[1],
            offset: 0.0,
            color: order.order_color,
            width: order.line_width,
        }
    }
}

pub(super) fn draw_order_label<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    order: &VisibleOrder,
    position: OrderLabelPosition,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let visual_label_y = ctx
        .fisheye
        .project(Point::new(ORDER_LABEL_X, position.label_y))
        .y;
    let (label_top, _) = order_label_y_range(visual_label_y);
    ctx.frame.fill_rectangle(
        Point::new(ORDER_LABEL_X, label_top),
        Size::new(order.side_label_width, ORDER_LABEL_HEIGHT),
        Color {
            a: 0.6,
            ..ctx.theme.extended_palette().background.strong.color
        },
    );
    ctx.frame.fill_text(canvas::Text {
        content: order.side_label.clone(),
        position: Point::new(order_label_text_x(order), visual_label_y),
        color: order.order_color_solid,
        size: iced::Pixels(8.0),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
    if order.pending_state.is_some() {
        draw_order_pending_spinner(ctx, order, visual_label_y);
        return;
    }

    ctx.frame.fill_rectangle(
        Point::new(order.cancel_x, label_top),
        Size::new(ORDER_CANCEL_WIDTH, ORDER_LABEL_HEIGHT),
        Color {
            a: 0.5,
            ..ctx.theme.palette().danger
        },
    );
    ctx.frame.fill_text(canvas::Text {
        content: "x".to_string(),
        position: Point::new(order.cancel_x + ORDER_CANCEL_WIDTH * 0.5, visual_label_y),
        color: Color::WHITE,
        size: iced::Pixels(8.0),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

fn order_label_text_x(order: &VisibleOrder) -> f32 {
    if order.pending_state.is_some() {
        ORDER_PENDING_TEXT_X
    } else {
        ORDER_LABEL_TEXT_X
    }
}

fn draw_order_pending_spinner<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    order: &VisibleOrder,
    center_y: f32,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let phase = if order.line_offset.is_finite() {
        order.line_offset * 0.5
    } else {
        0.0
    };
    let spinner = canvas::Path::new(|path| {
        path.arc(canvas::path::Arc {
            center: Point::new(ORDER_PENDING_SPINNER_X, center_y),
            radius: 3.0,
            start_angle: Radians(phase),
            end_angle: Radians(phase + std::f32::consts::PI * 1.45),
        });
    });
    ctx.frame.stroke(
        &spinner,
        canvas::Stroke::default()
            .with_color(order.order_color_solid)
            .with_width(1.15)
            .with_line_cap(canvas::LineCap::Round),
    );
}
