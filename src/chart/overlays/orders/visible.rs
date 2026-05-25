use super::super::TradingOverlayContext;
use crate::chart::OrderOverlayPendingState;
use crate::chart::model::CandlestickChart;
use crate::chart::order_labels::{
    ORDER_CANCEL_GAP, ORDER_CANCEL_WIDTH, ORDER_LABEL_X, order_side_label,
    order_side_label_width_for_order,
};
use crate::chart::state::DragKind;

use iced::Color;

// ---------------------------------------------------------------------------
// Visible Order Preparation
// ---------------------------------------------------------------------------

pub(super) const ORDER_LINE_WIDTH: f32 = 1.5;
pub(super) const MOVING_ORDER_LINE_WIDTH: f32 = 2.0;
pub(super) const ORDER_LINE_DASH: [f32; 2] = [3.0, 5.0];
pub(super) const MOVING_ORDER_LINE_DASH: [f32; 2] = [8.0, 4.0];

pub(super) struct VisibleOrder {
    pub(super) order_index: usize,
    pub(super) display_px: f64,
    pub(super) order_y: f32,
    pub(super) order_color: Color,
    pub(super) order_color_solid: Color,
    pub(super) line_width: f32,
    pub(super) line_offset: f32,
    pub(super) is_animating: bool,
    pub(super) is_buy: bool,
    pub(super) side_label: String,
    pub(super) side_label_width: f32,
    pub(super) cancel_x: f32,
    pub(super) label_right_x: f32,
    pub(super) pending_state: Option<OrderOverlayPendingState>,
}

pub(super) fn collect_visible_orders<PriceToY, IdxToCx>(
    chart: &CandlestickChart,
    ctx: &TradingOverlayContext<'_, PriceToY, IdxToCx>,
) -> Vec<VisibleOrder>
where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let dragging_oid = match ctx.state.drag {
        Some(DragKind::MoveOrder { oid }) => Some(oid),
        _ => None,
    };

    let mut visible_orders = Vec::with_capacity(chart.active_orders.len());

    for (order_index, order) in chart.active_orders.iter().enumerate() {
        let is_dragging = dragging_oid == Some(order.oid);
        let is_pending = order.pending_state.is_some();
        let pending_animates = matches!(
            order.pending_state,
            Some(OrderOverlayPendingState::Cancelling | OrderOverlayPendingState::Modifying)
        );
        let is_animating = is_dragging || order.is_moving || pending_animates;
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

        let (order_color, order_color_solid, line_width) = visible_order_style(
            ctx,
            is_dragging || order.is_moving || pending_animates,
            order.is_buy,
        );
        let side_label = order_side_label(order);
        let side_label_width = order_side_label_width_for_order(order, &side_label);
        let cancel_x = ORDER_LABEL_X + side_label_width + ORDER_CANCEL_GAP;
        let label_right_x = if is_pending {
            ORDER_LABEL_X + side_label_width
        } else {
            cancel_x + ORDER_CANCEL_WIDTH
        };

        visible_orders.push(VisibleOrder {
            order_index,
            display_px,
            order_y,
            order_color,
            order_color_solid,
            line_width,
            line_offset: chart.order_line_phase,
            is_animating,
            is_buy: order.is_buy,
            side_label,
            side_label_width,
            cancel_x,
            label_right_x,
            pending_state: order.pending_state,
        });
    }

    visible_orders
}

fn visible_order_style<PriceToY, IdxToCx>(
    ctx: &TradingOverlayContext<'_, PriceToY, IdxToCx>,
    is_dragging: bool,
    is_buy: bool,
) -> (Color, Color, f32)
where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    if is_dragging {
        if is_buy {
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
    } else if is_buy {
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
    }
}
