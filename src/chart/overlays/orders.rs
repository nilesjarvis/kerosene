use super::TradingOverlayContext;
use crate::chart::drawing::{AxisBadgeStyle, SegmentedHLineStyle};
use crate::chart::model::CandlestickChart;
use crate::chart::order_labels::{
    ORDER_CANCEL_GAP, ORDER_CANCEL_WIDTH, ORDER_LABEL_CONNECTOR_SPAN, ORDER_LABEL_HEIGHT,
    ORDER_LABEL_TEXT_X, ORDER_LABEL_X, OrderLabelAnchor, OrderLabelPosition, order_label_position,
    order_label_position_slots, order_label_y_range, order_side_label,
    order_side_label_width_for_label, stack_order_label_positions_avoiding,
};
use crate::chart::price_badges::{
    RIGHT_AXIS_SECONDARY_BADGE_HEIGHT, RightAxisBadgeConnectorStyle, RightAxisBadgeKind,
    draw_stacked_right_axis_badge, right_axis_line_end_x,
};
use crate::chart::state::DragKind;
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Size, alignment};

// ---------------------------------------------------------------------------
// Order Overlays
// ---------------------------------------------------------------------------

const ORDER_LINE_WIDTH: f32 = 1.5;
const MOVING_ORDER_LINE_WIDTH: f32 = 2.0;
const ORDER_LINE_DASH: [f32; 2] = [3.0, 5.0];
const MOVING_ORDER_LINE_DASH: [f32; 2] = [8.0, 4.0];

struct VisibleOrder {
    order_index: usize,
    display_px: f64,
    order_y: f32,
    order_color: Color,
    order_color_solid: Color,
    line_width: f32,
    line_offset: f32,
    is_animating: bool,
    is_buy: bool,
    side_label: String,
    side_label_width: f32,
    cancel_x: f32,
    label_right_x: f32,
}

impl CandlestickChart {
    pub(super) fn draw_active_order_lines<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if ctx.price_range <= 0.0
            || !ctx.price_range.is_finite()
            || ctx.chart_w <= 0.0
            || ctx.price_h <= 0.0
            || !ctx.chart_w.is_finite()
            || !ctx.price_h.is_finite()
        {
            return;
        }

        let dragging_oid = match ctx.state.drag {
            Some(DragKind::MoveOrder { oid }) => Some(oid),
            _ => None,
        };

        let mut visible_orders = Vec::with_capacity(self.active_orders.len());

        for (order_index, order) in self.active_orders.iter().enumerate() {
            let is_dragging = dragging_oid == Some(order.oid);
            let is_animating = is_dragging || order.is_moving;
            let display_px = if is_dragging {
                ctx.state.drag_order_new_price.unwrap_or(order.limit_px)
            } else {
                order.limit_px
            };
            if !display_px.is_finite() {
                continue;
            }

            let order_y = (ctx.price_to_y)(display_px);
            if order_y < -10.0 || order_y > ctx.price_h + 10.0 || !order_y.is_finite() {
                continue;
            }

            let (order_color, order_color_solid, line_width) = if is_dragging {
                if order.is_buy {
                    (
                        Color {
                            a: 0.60,
                            ..ctx.theme.palette().success
                        },
                        ctx.theme.palette().success,
                        MOVING_ORDER_LINE_WIDTH,
                    )
                } else {
                    (
                        Color {
                            a: 0.60,
                            ..ctx.theme.palette().danger
                        },
                        ctx.theme.palette().danger,
                        MOVING_ORDER_LINE_WIDTH,
                    )
                }
            } else if order.is_buy {
                (
                    Color {
                        a: 0.35,
                        ..ctx.theme.palette().success
                    },
                    ctx.theme.palette().success,
                    ORDER_LINE_WIDTH,
                )
            } else {
                (
                    Color {
                        a: 0.35,
                        ..ctx.theme.palette().danger
                    },
                    ctx.theme.palette().danger,
                    ORDER_LINE_WIDTH,
                )
            };
            let side_label = order_side_label(order);
            let side_label_width = order_side_label_width_for_label(&side_label);
            let cancel_x = ORDER_LABEL_X + side_label_width + ORDER_CANCEL_GAP;
            let label_right_x = cancel_x + ORDER_CANCEL_WIDTH;

            visible_orders.push(VisibleOrder {
                order_index,
                display_px,
                order_y,
                order_color,
                order_color_solid,
                line_width,
                line_offset: self.order_line_phase,
                is_animating,
                is_buy: order.is_buy,
                side_label,
                side_label_width,
                cancel_x,
                label_right_x,
            });
        }

        let reserved_ranges = self.order_label_reserved_ranges(ctx.price_h, ctx.price_to_y);
        let label_positions = order_label_position_slots(
            stack_order_label_positions_avoiding(
                visible_orders
                    .iter()
                    .map(|order| OrderLabelAnchor {
                        order_index: order.order_index,
                        order_y: order.order_y,
                        is_buy: order.is_buy,
                    })
                    .collect(),
                ctx.price_h,
                &reserved_ranges,
            ),
            self.active_orders.len(),
        );

        for visible_order in &visible_orders {
            if let Some(position) =
                order_label_position(&label_positions, visible_order.order_index)
            {
                draw_order_line(ctx, visible_order, position);
                draw_order_price_badge(ctx, visible_order);
            }
        }

        for visible_order in &visible_orders {
            if let Some(position) =
                order_label_position(&label_positions, visible_order.order_index)
            {
                draw_order_label_connector(ctx, visible_order, position);
            }
        }

        for visible_order in &visible_orders {
            if let Some(position) =
                order_label_position(&label_positions, visible_order.order_index)
            {
                draw_order_label(ctx, visible_order, position);
            }
        }
    }
}

fn draw_order_line<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    order: &VisibleOrder,
    position: OrderLabelPosition,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let start_x = order_line_start_x(order, position, ctx.chart_w);
    let badge_kind = RightAxisBadgeKind::ActiveOrder(order.order_index);
    let end_x = right_axis_line_end_x(
        ctx.right_axis_badges,
        badge_kind,
        order.order_y,
        ctx.chart_w,
    );
    let style = order_line_style(order);
    stroke_segmented_hline_range(ctx.frame, start_x, end_x, order.order_y, style);
}

fn draw_order_price_badge<PriceToY, IdxToCx>(
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
    );
}

fn draw_order_label_connector<PriceToY, IdxToCx>(
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

    let mut has_segment = false;
    let connector = canvas::Path::new(|path| {
        has_segment = append_segmented_quadratic_curve(
            path,
            Point::new(start_x, order.order_y),
            Point::new(
                order.label_right_x + (start_x - order.label_right_x) * 0.45,
                order.order_y,
            ),
            Point::new(order.label_right_x, position.label_y),
            &order_line_style(order),
        );
    });

    if has_segment {
        ctx.frame
            .stroke(&connector, order_solid_stroke(&order_line_style(order)));
    }
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

fn stroke_segmented_hline_range(
    frame: &mut canvas::Frame,
    start_x: f32,
    end_x: f32,
    y: f32,
    style: SegmentedHLineStyle,
) {
    if end_x <= start_x
        || style.segment_len <= 0.0
        || !start_x.is_finite()
        || !end_x.is_finite()
        || !y.is_finite()
        || !style.segment_len.is_finite()
        || !style.gap_len.is_finite()
    {
        return;
    }

    let stride = style.segment_len + style.gap_len.max(0.0);
    if stride <= 0.0 || !stride.is_finite() {
        return;
    }

    let phase = if style.offset.is_finite() {
        style.offset.rem_euclid(stride)
    } else {
        0.0
    };

    let mut has_segment = false;
    let line = canvas::Path::new(|path| {
        let mut x = phase - stride;
        while x < end_x {
            let start = x.max(start_x);
            let end = (x + style.segment_len).min(end_x);
            if end > start {
                path.move_to(Point::new(start, y));
                path.line_to(Point::new(end, y));
                has_segment = true;
            }
            x += stride;
        }
    });

    if has_segment {
        frame.stroke(&line, order_solid_stroke(&style));
    }
}

fn append_segmented_quadratic_curve(
    path: &mut canvas::path::Builder,
    start: Point,
    control: Point,
    end: Point,
    style: &SegmentedHLineStyle,
) -> bool {
    const SAMPLES: usize = 18;

    if !valid_point(start)
        || !valid_point(control)
        || !valid_point(end)
        || style.segment_len <= 0.0
        || !style.segment_len.is_finite()
        || !style.gap_len.is_finite()
    {
        return false;
    }

    let stride = style.segment_len + style.gap_len.max(0.0);
    if stride <= 0.0 || !stride.is_finite() {
        return false;
    }

    let phase = if style.offset.is_finite() {
        style.offset.rem_euclid(stride)
    } else {
        0.0
    };
    let mut next_dash_start = phase - stride;
    while next_dash_start + style.segment_len <= 0.0 {
        next_dash_start += stride;
    }

    let mut previous = start;
    let mut previous_distance = 0.0;
    let mut has_segment = false;
    for sample in 1..=SAMPLES {
        let t = sample as f32 / SAMPLES as f32;
        let current = quadratic_point(start, control, end, t);
        let segment_len = distance(previous, current);
        if segment_len > 0.0 && segment_len.is_finite() {
            let segment_start_distance = previous_distance;
            let segment_end_distance = previous_distance + segment_len;

            while next_dash_start < segment_end_distance {
                let dash_start = next_dash_start.max(segment_start_distance);
                let dash_end = (next_dash_start + style.segment_len).min(segment_end_distance);
                if dash_end > dash_start {
                    let start_t = (dash_start - segment_start_distance) / segment_len;
                    let end_t = (dash_end - segment_start_distance) / segment_len;
                    path.move_to(lerp_point(previous, current, start_t));
                    path.line_to(lerp_point(previous, current, end_t));
                    has_segment = true;
                }
                next_dash_start += stride;
            }

            previous_distance = segment_end_distance;
        }
        previous = current;
    }
    has_segment
}

fn quadratic_point(start: Point, control: Point, end: Point, t: f32) -> Point {
    let inv_t = 1.0 - t;
    let start_weight = inv_t * inv_t;
    let control_weight = 2.0 * inv_t * t;
    let end_weight = t * t;
    Point::new(
        start.x * start_weight + control.x * control_weight + end.x * end_weight,
        start.y * start_weight + control.y * control_weight + end.y * end_weight,
    )
}

fn lerp_point(start: Point, end: Point, t: f32) -> Point {
    Point::new(
        start.x + (end.x - start.x) * t,
        start.y + (end.y - start.y) * t,
    )
}

fn distance(start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    (dx * dx + dy * dy).sqrt()
}

fn valid_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn order_solid_stroke(style: &SegmentedHLineStyle) -> canvas::Stroke<'static> {
    canvas::Stroke::default()
        .with_color(style.color)
        .with_width(style.width)
}

fn draw_order_label<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    order: &VisibleOrder,
    position: OrderLabelPosition,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let (label_top, _) = order_label_y_range(position.label_y);
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
        position: Point::new(ORDER_LABEL_TEXT_X, position.label_y),
        color: order.order_color_solid,
        size: iced::Pixels(8.0),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });

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
        position: Point::new(order.cancel_x + ORDER_CANCEL_WIDTH * 0.5, position.label_y),
        color: Color::WHITE,
        size: iced::Pixels(8.0),
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}
