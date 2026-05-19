use super::drawing::{AxisBadgeStyle, SegmentedHLineStyle, fill_right_axis_badge};
use super::model::CandlestickChart;
use super::state::{ChartState, DragKind};
use crate::annotations::AnnotationKind;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// Right Axis Price Badge Layout
// ---------------------------------------------------------------------------

pub(super) const RIGHT_AXIS_PRIMARY_BADGE_HEIGHT: f32 = 16.0;
pub(super) const RIGHT_AXIS_SECONDARY_BADGE_HEIGHT: f32 = 14.0;

const RIGHT_AXIS_BADGE_GAP: f32 = 2.0;
const RIGHT_AXIS_BADGE_MARGIN: f32 = 2.0;
const RIGHT_AXIS_BADGE_CONNECTOR_SPAN: f32 = 18.0;
const RIGHT_AXIS_BADGE_CONNECTOR_SHIFT_EPSILON: f32 = 1.0;
const RIGHT_AXIS_SELL_ORDER_SORT_BASE: usize = 20_000;
const RIGHT_AXIS_CURRENT_PRICE_SORT_RANK: usize = 40_000;
const RIGHT_AXIS_POSITION_ENTRY_SORT_RANK: usize = 50_000;
const RIGHT_AXIS_QUICK_ORDER_SORT_RANK: usize = 60_000;
const RIGHT_AXIS_LIQUIDATION_SORT_RANK: usize = 70_000;
const RIGHT_AXIS_BUY_ORDER_SORT_BASE: usize = 80_000;
const RIGHT_AXIS_ANNOTATION_SORT_BASE: usize = 90_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixedBadgeSide {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RightAxisBadgeKind {
    CurrentPrice,
    QuickOrder,
    PositionEntry,
    PositionLiquidation,
    ActiveOrder(usize),
    HorizontalAnnotation(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RightAxisBadgeAnchor {
    kind: RightAxisBadgeKind,
    source_y: f32,
    height: f32,
    sort_rank: usize,
    fixed_side: Option<FixedBadgeSide>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RightAxisBadgePosition {
    pub(super) source_y: f32,
    pub(super) badge_y: f32,
}

#[derive(Debug, Clone)]
pub(super) struct RightAxisBadgeLayout {
    current_price: Option<RightAxisBadgePosition>,
    quick_order: Option<RightAxisBadgePosition>,
    position_entry: Option<RightAxisBadgePosition>,
    position_liquidation: Option<RightAxisBadgePosition>,
    active_orders: Vec<Option<RightAxisBadgePosition>>,
    horizontal_annotations: Vec<Option<RightAxisBadgePosition>>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum RightAxisBadgeConnectorStyle {
    Solid {
        color: Color,
        width: f32,
    },
    Segmented {
        style: SegmentedHLineStyle,
    },
}

impl CandlestickChart {
    pub(super) fn right_axis_badge_layout<PriceToY>(
        &self,
        state: &ChartState,
        price_h: f32,
        price_range: f64,
        price_to_y: &PriceToY,
    ) -> RightAxisBadgeLayout
    where
        PriceToY: Fn(f64) -> f32,
    {
        let mut layout =
            RightAxisBadgeLayout::empty(self.active_orders.len(), self.annotations.len());
        if price_range <= 0.0
            || price_h <= 0.0
            || !price_range.is_finite()
            || !price_h.is_finite()
        {
            return layout;
        }

        let mut anchors =
            Vec::with_capacity(self.active_orders.len() + self.annotations.len() + 4);

        if let Some(last_candle) = self.candles.last() {
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

        if let Some(price) = self.quick_order_limit_price
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

        if !self.hide_positions_and_orders
            && !self.obscure_position_prices
            && let Some(position) = &self.active_position
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

        if !self.hide_positions_and_orders {
            let dragging_oid = match state.drag {
                Some(DragKind::MoveOrder { oid }) => Some(oid),
                _ => None,
            };
            for (order_index, order) in self.active_orders.iter().enumerate() {
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

        for (annotation_index, annotation) in self.annotations.iter().enumerate() {
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

        for stacked in stack_right_axis_badge_positions(anchors, price_h) {
            layout.insert(stacked);
        }

        layout
    }
}

impl RightAxisBadgeLayout {
    fn empty(active_order_count: usize, annotation_count: usize) -> Self {
        Self {
            current_price: None,
            quick_order: None,
            position_entry: None,
            position_liquidation: None,
            active_orders: vec![None; active_order_count],
            horizontal_annotations: vec![None; annotation_count],
        }
    }

    pub(super) fn position(&self, kind: RightAxisBadgeKind) -> Option<RightAxisBadgePosition> {
        match kind {
            RightAxisBadgeKind::CurrentPrice => self.current_price,
            RightAxisBadgeKind::QuickOrder => self.quick_order,
            RightAxisBadgeKind::PositionEntry => self.position_entry,
            RightAxisBadgeKind::PositionLiquidation => self.position_liquidation,
            RightAxisBadgeKind::ActiveOrder(index) => {
                self.active_orders.get(index).copied().flatten()
            }
            RightAxisBadgeKind::HorizontalAnnotation(index) => {
                self.horizontal_annotations.get(index).copied().flatten()
            }
        }
    }

    fn insert(&mut self, stacked: StackedRightAxisBadge) {
        let position = RightAxisBadgePosition {
            source_y: stacked.source_y,
            badge_y: stacked.badge_y,
        };
        match stacked.kind {
            RightAxisBadgeKind::CurrentPrice => self.current_price = Some(position),
            RightAxisBadgeKind::QuickOrder => self.quick_order = Some(position),
            RightAxisBadgeKind::PositionEntry => self.position_entry = Some(position),
            RightAxisBadgeKind::PositionLiquidation => {
                self.position_liquidation = Some(position);
            }
            RightAxisBadgeKind::ActiveOrder(index) => {
                if let Some(slot) = self.active_orders.get_mut(index) {
                    *slot = Some(position);
                }
            }
            RightAxisBadgeKind::HorizontalAnnotation(index) => {
                if let Some(slot) = self.horizontal_annotations.get_mut(index) {
                    *slot = Some(position);
                }
            }
        }
    }
}

pub(super) fn right_axis_line_end_x(
    layout: &RightAxisBadgeLayout,
    kind: RightAxisBadgeKind,
    source_y: f32,
    chart_w: f32,
) -> f32 {
    if chart_w <= 0.0 || !chart_w.is_finite() {
        return 0.0;
    }

    if let Some(position) = layout.position(kind)
        && (position.badge_y - source_y).abs() >= RIGHT_AXIS_BADGE_CONNECTOR_SHIFT_EPSILON
    {
        return (chart_w - RIGHT_AXIS_BADGE_CONNECTOR_SPAN).max(0.0);
    }

    chart_w
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_stacked_right_axis_badge(
    frame: &mut canvas::Frame,
    layout: &RightAxisBadgeLayout,
    kind: RightAxisBadgeKind,
    chart_w: f32,
    source_y: f32,
    label: String,
    background: Color,
    badge_style: AxisBadgeStyle,
    connector_style: RightAxisBadgeConnectorStyle,
) {
    let badge_y = layout
        .position(kind)
        .map_or(source_y, |position| position.badge_y);
    if (badge_y - source_y).abs() >= RIGHT_AXIS_BADGE_CONNECTOR_SHIFT_EPSILON {
        let start_x = right_axis_line_end_x(layout, kind, source_y, chart_w);
        let start = Point::new(start_x, source_y);
        let control = Point::new(start_x + (chart_w - start_x) * 0.55, source_y);
        let end = Point::new(chart_w + 1.0, badge_y);
        stroke_right_axis_connector(frame, start, control, end, connector_style);
    }

    fill_right_axis_badge(frame, chart_w, badge_y, label, background, badge_style);
}

fn push_visible_badge(
    anchors: &mut Vec<RightAxisBadgeAnchor>,
    kind: RightAxisBadgeKind,
    source_y: f32,
    height: f32,
    sort_rank: usize,
    fixed_side: Option<FixedBadgeSide>,
    price_h: f32,
) {
    if source_y >= -10.0
        && source_y <= price_h + 10.0
        && source_y.is_finite()
        && height > 0.0
        && height.is_finite()
    {
        anchors.push(RightAxisBadgeAnchor {
            kind,
            source_y,
            height,
            sort_rank,
            fixed_side,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct StackedRightAxisBadge {
    kind: RightAxisBadgeKind,
    source_y: f32,
    badge_y: f32,
    height: f32,
}

fn stack_right_axis_badge_positions(
    mut anchors: Vec<RightAxisBadgeAnchor>,
    price_h: f32,
) -> Vec<StackedRightAxisBadge> {
    anchors.retain(|anchor| {
        anchor.source_y.is_finite()
            && anchor.height > 0.0
            && anchor.height.is_finite()
    });
    if anchors.is_empty() || price_h <= 0.0 || !price_h.is_finite() {
        return Vec::new();
    }

    if let Some(position_index) = anchors
        .iter()
        .position(|anchor| anchor.kind == RightAxisBadgeKind::PositionEntry)
    {
        return stack_right_axis_badges_around_fixed_position(anchors, position_index, price_h);
    }

    anchors.sort_by(|a, b| {
        a.source_y
            .total_cmp(&b.source_y)
            .then_with(|| a.sort_rank.cmp(&b.sort_rank))
    });

    let min_top = RIGHT_AXIS_BADGE_MARGIN;
    let max_bottom = (price_h - RIGHT_AXIS_BADGE_MARGIN).max(min_top);
    stack_right_axis_badges_in_band(anchors, min_top, max_bottom, true, true)
}

fn stack_right_axis_badges_around_fixed_position(
    mut anchors: Vec<RightAxisBadgeAnchor>,
    position_index: usize,
    price_h: f32,
) -> Vec<StackedRightAxisBadge> {
    let position = anchors.remove(position_index);
    let fixed = StackedRightAxisBadge {
        kind: position.kind,
        source_y: position.source_y,
        badge_y: position.source_y,
        height: position.height,
    };
    let fixed_top = badge_top(fixed);
    let fixed_bottom = badge_bottom(fixed);
    let min_top = RIGHT_AXIS_BADGE_MARGIN;
    let max_bottom = (price_h - RIGHT_AXIS_BADGE_MARGIN).max(min_top);
    let above_max_bottom = fixed_top - RIGHT_AXIS_BADGE_GAP;
    let below_min_top = fixed_bottom + RIGHT_AXIS_BADGE_GAP;
    let mut above = Vec::new();
    let mut below = Vec::new();

    for anchor in anchors {
        match preferred_side_of_fixed_position(anchor, fixed_top, fixed_bottom) {
            FixedBadgeSide::Above => above.push(anchor),
            FixedBadgeSide::Below => below.push(anchor),
        }
    }

    let mut positions =
        stack_right_axis_badges_in_band(above, min_top, above_max_bottom, false, true);
    positions.push(fixed);
    positions.extend(stack_right_axis_badges_in_band(
        below,
        below_min_top,
        max_bottom,
        true,
        false,
    ));
    positions.sort_by(|a, b| {
        a.badge_y
            .total_cmp(&b.badge_y)
            .then_with(|| badge_sort_rank(a.kind).cmp(&badge_sort_rank(b.kind)))
    });
    positions
}

fn preferred_side_of_fixed_position(
    anchor: RightAxisBadgeAnchor,
    fixed_top: f32,
    fixed_bottom: f32,
) -> FixedBadgeSide {
    let anchor_top = anchor.source_y - anchor.height * 0.5;
    let anchor_bottom = anchor.source_y + anchor.height * 0.5;
    if anchor_bottom + RIGHT_AXIS_BADGE_GAP <= fixed_top {
        FixedBadgeSide::Above
    } else if anchor_top - RIGHT_AXIS_BADGE_GAP >= fixed_bottom {
        FixedBadgeSide::Below
    } else if let Some(side) = anchor.fixed_side {
        side
    } else if anchor.source_y <= (fixed_top + fixed_bottom) * 0.5 {
        FixedBadgeSide::Above
    } else {
        FixedBadgeSide::Below
    }
}

fn stack_right_axis_badges_in_band(
    mut anchors: Vec<RightAxisBadgeAnchor>,
    min_top: f32,
    max_bottom: f32,
    protect_top: bool,
    protect_bottom: bool,
) -> Vec<StackedRightAxisBadge> {
    if anchors.is_empty() {
        return Vec::new();
    }

    anchors.sort_by(|a, b| {
        a.source_y
            .total_cmp(&b.source_y)
            .then_with(|| a.sort_rank.cmp(&b.sort_rank))
    });

    let mut positions = Vec::with_capacity(anchors.len());
    let mut next_top = min_top;

    for anchor in anchors {
        let max_top = (max_bottom - anchor.height).max(min_top);
        let desired_top = anchor.source_y - anchor.height * 0.5;
        let top = desired_top.clamp(min_top, max_top).max(next_top);
        positions.push(StackedRightAxisBadge {
            kind: anchor.kind,
            source_y: anchor.source_y,
            badge_y: top + anchor.height * 0.5,
            height: anchor.height,
        });
        next_top = top + anchor.height + RIGHT_AXIS_BADGE_GAP;
    }

    if protect_bottom
        && positions
            .last()
            .is_some_and(|position| badge_bottom(*position) > max_bottom)
    {
        let mut next_bottom = max_bottom;
        for position in positions.iter_mut().rev() {
            let current_top = badge_top(*position);
            let max_top = if protect_top {
                (next_bottom - position.height).max(min_top)
            } else {
                next_bottom - position.height
            };
            let top = current_top.min(max_top);
            position.badge_y = top + position.height * 0.5;
            next_bottom = top - RIGHT_AXIS_BADGE_GAP;
        }

        if protect_top
            && let Some(first) = positions.first()
            && badge_top(*first) < min_top
        {
            let shift = min_top - badge_top(*first);
            for position in &mut positions {
                position.badge_y += shift;
            }
        }
    }

    positions
}

fn badge_sort_rank(kind: RightAxisBadgeKind) -> usize {
    match kind {
        RightAxisBadgeKind::CurrentPrice => RIGHT_AXIS_CURRENT_PRICE_SORT_RANK,
        RightAxisBadgeKind::QuickOrder => RIGHT_AXIS_QUICK_ORDER_SORT_RANK,
        RightAxisBadgeKind::PositionEntry => RIGHT_AXIS_POSITION_ENTRY_SORT_RANK,
        RightAxisBadgeKind::PositionLiquidation => RIGHT_AXIS_LIQUIDATION_SORT_RANK,
        RightAxisBadgeKind::ActiveOrder(index) => RIGHT_AXIS_SELL_ORDER_SORT_BASE + index,
        RightAxisBadgeKind::HorizontalAnnotation(index) => RIGHT_AXIS_ANNOTATION_SORT_BASE + index,
    }
}

fn badge_top(position: StackedRightAxisBadge) -> f32 {
    position.badge_y - position.height * 0.5
}

fn badge_bottom(position: StackedRightAxisBadge) -> f32 {
    position.badge_y + position.height * 0.5
}

fn stroke_right_axis_connector(
    frame: &mut canvas::Frame,
    start: Point,
    control: Point,
    end: Point,
    style: RightAxisBadgeConnectorStyle,
) {
    match style {
        RightAxisBadgeConnectorStyle::Solid { color, width } => {
            let Some(path) = solid_quadratic_path(start, control, end) else {
                return;
            };
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(width)
                    .with_line_cap(canvas::LineCap::Round),
            );
        }
        RightAxisBadgeConnectorStyle::Segmented { style } => {
            let mut has_segment = false;
            let connector = canvas::Path::new(|path| {
                has_segment = append_segmented_quadratic_curve(path, start, control, end, &style);
            });
            if has_segment {
                frame.stroke(
                    &connector,
                    canvas::Stroke::default()
                        .with_color(style.color)
                        .with_width(style.width)
                        .with_line_cap(canvas::LineCap::Round),
                );
            }
        }
    }
}

fn solid_quadratic_path(start: Point, control: Point, end: Point) -> Option<canvas::Path> {
    const SAMPLES: usize = 14;

    if !valid_point(start) || !valid_point(control) || !valid_point(end) {
        return None;
    }

    Some(canvas::Path::new(|path| {
        path.move_to(start);
        for sample in 1..=SAMPLES {
            let t = sample as f32 / SAMPLES as f32;
            path.line_to(quadratic_point(start, control, end, t));
        }
    }))
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

fn distance(a: Point, b: Point) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

fn valid_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_axis_badges_stack_nearby_labels() {
        let positions = stack_right_axis_badge_positions(
            vec![
                anchor(RightAxisBadgeKind::CurrentPrice, 40.0, 16.0, 0, None),
                anchor(RightAxisBadgeKind::QuickOrder, 42.0, 16.0, 1, None),
                anchor(RightAxisBadgeKind::ActiveOrder(0), 43.0, 14.0, 2, None),
            ],
            120.0,
        );

        assert_eq!(positions.len(), 3);
        assert_non_overlapping(&positions);
    }

    #[test]
    fn right_axis_badges_pack_back_inside_bottom_edge() {
        let positions = stack_right_axis_badge_positions(
            vec![
                anchor(RightAxisBadgeKind::CurrentPrice, 96.0, 16.0, 0, None),
                anchor(RightAxisBadgeKind::QuickOrder, 98.0, 16.0, 1, None),
            ],
            100.0,
        );

        assert_eq!(positions.len(), 2);
        assert_non_overlapping(&positions);
        assert!(badge_bottom(positions[1]) <= 98.0);
    }

    #[test]
    fn right_axis_order_tie_keeps_sells_above_buys() {
        let positions = stack_right_axis_badge_positions(
            vec![
                anchor(
                    RightAxisBadgeKind::ActiveOrder(0),
                    50.0,
                    14.0,
                    RIGHT_AXIS_BUY_ORDER_SORT_BASE,
                    Some(FixedBadgeSide::Below),
                ),
                anchor(
                    RightAxisBadgeKind::PositionEntry,
                    50.0,
                    16.0,
                    RIGHT_AXIS_POSITION_ENTRY_SORT_RANK,
                    None,
                ),
                anchor(
                    RightAxisBadgeKind::ActiveOrder(1),
                    50.0,
                    14.0,
                    RIGHT_AXIS_SELL_ORDER_SORT_BASE,
                    Some(FixedBadgeSide::Above),
                ),
            ],
            160.0,
        );

        assert_eq!(positions[0].kind, RightAxisBadgeKind::ActiveOrder(1));
        assert_eq!(positions[1].kind, RightAxisBadgeKind::PositionEntry);
        assert_eq!(positions[2].kind, RightAxisBadgeKind::ActiveOrder(0));
        assert_eq!(positions[1].badge_y, 50.0);
        assert_non_overlapping(&positions);
    }

    #[test]
    fn right_axis_position_entry_stays_fixed_when_limit_orders_overlap() {
        let positions = stack_right_axis_badge_positions(
            vec![
                anchor(
                    RightAxisBadgeKind::PositionEntry,
                    50.0,
                    16.0,
                    RIGHT_AXIS_POSITION_ENTRY_SORT_RANK,
                    None,
                ),
                anchor(
                    RightAxisBadgeKind::ActiveOrder(0),
                    50.0,
                    14.0,
                    RIGHT_AXIS_SELL_ORDER_SORT_BASE,
                    Some(FixedBadgeSide::Above),
                ),
                anchor(
                    RightAxisBadgeKind::ActiveOrder(1),
                    50.0,
                    14.0,
                    RIGHT_AXIS_BUY_ORDER_SORT_BASE,
                    Some(FixedBadgeSide::Below),
                ),
            ],
            120.0,
        );
        let position = positions
            .iter()
            .find(|position| position.kind == RightAxisBadgeKind::PositionEntry)
            .copied()
            .expect("fixed position badge");
        let sell = positions
            .iter()
            .find(|position| position.kind == RightAxisBadgeKind::ActiveOrder(0))
            .copied()
            .expect("sell order badge");
        let buy = positions
            .iter()
            .find(|position| position.kind == RightAxisBadgeKind::ActiveOrder(1))
            .copied()
            .expect("buy order badge");

        assert_eq!(position.badge_y, 50.0);
        assert!(badge_bottom(sell) + RIGHT_AXIS_BADGE_GAP <= badge_top(position));
        assert!(badge_top(buy) - RIGHT_AXIS_BADGE_GAP >= badge_bottom(position));
    }

    fn anchor(
        kind: RightAxisBadgeKind,
        source_y: f32,
        height: f32,
        sort_rank: usize,
        fixed_side: Option<FixedBadgeSide>,
    ) -> RightAxisBadgeAnchor {
        RightAxisBadgeAnchor {
            kind,
            source_y,
            height,
            sort_rank,
            fixed_side,
        }
    }

    fn assert_non_overlapping(positions: &[StackedRightAxisBadge]) {
        for pair in positions.windows(2) {
            assert!(
                badge_bottom(pair[0]) + RIGHT_AXIS_BADGE_GAP <= badge_top(pair[1]),
                "{:?} overlaps {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}
