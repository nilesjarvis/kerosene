use super::super::super::model::CandlestickChart;
use super::super::super::state::{ChartState, DragKind};
use super::stacking::{
    FixedBadgeSide, RIGHT_AXIS_ANNOTATION_SORT_BASE, RIGHT_AXIS_BUY_ORDER_SORT_BASE,
    RIGHT_AXIS_CURRENT_PRICE_SORT_RANK, RIGHT_AXIS_LIQUIDATION_SORT_RANK,
    RIGHT_AXIS_POSITION_ENTRY_SORT_RANK, RIGHT_AXIS_QUICK_ORDER_SORT_RANK,
    RIGHT_AXIS_SELL_ORDER_SORT_BASE, RightAxisBadgeAnchor, push_visible_badge,
};
use super::{
    RIGHT_AXIS_PRIMARY_BADGE_HEIGHT, RIGHT_AXIS_SECONDARY_BADGE_HEIGHT, RightAxisBadgeKind,
};
use crate::annotations::AnnotationKind;

// ---------------------------------------------------------------------------
// Right Axis Badge Anchors
// ---------------------------------------------------------------------------

pub(super) fn right_axis_badge_anchors<PriceToY>(
    chart: &CandlestickChart,
    state: &ChartState,
    price_h: f32,
    price_to_y: &PriceToY,
) -> Vec<RightAxisBadgeAnchor>
where
    PriceToY: Fn(f64) -> f32,
{
    let mut anchors = Vec::with_capacity(chart.active_orders.len() + chart.annotations.len() + 4);

    if let Some(last_candle) = chart.candles.last() {
        push_visible_badge(
            &mut anchors,
            RightAxisBadgeKind::CurrentPrice,
            price_to_y(last_candle.close),
            RIGHT_AXIS_PRIMARY_BADGE_HEIGHT,
            RIGHT_AXIS_CURRENT_PRICE_SORT_RANK,
            None,
            price_h,
        );
    }

    if let Some(price) = chart.quick_order_limit_price
        && price.is_finite()
        && price > 0.0
    {
        push_visible_badge(
            &mut anchors,
            RightAxisBadgeKind::QuickOrder,
            price_to_y(price),
            RIGHT_AXIS_PRIMARY_BADGE_HEIGHT,
            RIGHT_AXIS_QUICK_ORDER_SORT_RANK,
            None,
            price_h,
        );
    }

    if !chart.hide_positions_and_orders
        && !chart.obscure_position_prices
        && let Some(position) = &chart.active_position
    {
        push_visible_badge(
            &mut anchors,
            RightAxisBadgeKind::PositionEntry,
            price_to_y(position.entry_px),
            RIGHT_AXIS_PRIMARY_BADGE_HEIGHT,
            RIGHT_AXIS_POSITION_ENTRY_SORT_RANK,
            None,
            price_h,
        );

        if let Some(liq_px) = position.liquidation_px {
            push_visible_badge(
                &mut anchors,
                RightAxisBadgeKind::PositionLiquidation,
                price_to_y(liq_px),
                RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
                RIGHT_AXIS_LIQUIDATION_SORT_RANK,
                None,
                price_h,
            );
        }
    }

    if !chart.hide_positions_and_orders {
        let dragging_oid = match state.drag {
            Some(DragKind::MoveOrder { oid }) => Some(oid),
            _ => None,
        };
        for (order_index, order) in chart.active_orders.iter().enumerate() {
            let display_px = if dragging_oid == Some(order.oid) {
                state.drag_order_new_price.unwrap_or(order.limit_px)
            } else {
                order.limit_px
            };
            if !display_px.is_finite() {
                continue;
            }

            let sort_rank = if order.is_buy {
                RIGHT_AXIS_BUY_ORDER_SORT_BASE + order_index
            } else {
                RIGHT_AXIS_SELL_ORDER_SORT_BASE + order_index
            };
            push_visible_badge(
                &mut anchors,
                RightAxisBadgeKind::ActiveOrder(order_index),
                price_to_y(display_px),
                RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
                sort_rank,
                Some(if order.is_buy {
                    FixedBadgeSide::Below
                } else {
                    FixedBadgeSide::Above
                }),
                price_h,
            );
        }
    }

    for (annotation_index, annotation) in chart.annotations.iter().enumerate() {
        if let AnnotationKind::HorizontalLevel { price } = &annotation.kind {
            push_visible_badge(
                &mut anchors,
                RightAxisBadgeKind::HorizontalAnnotation(annotation_index),
                price_to_y(*price),
                RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
                RIGHT_AXIS_ANNOTATION_SORT_BASE + annotation_index,
                None,
                price_h,
            );
        }
    }

    anchors
}
