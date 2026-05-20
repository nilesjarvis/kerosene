use super::drawing::{AxisBadgeStyle, SegmentedHLineStyle, stroke_segmented_hline};
use super::model::CandlestickChart;
use super::price_badges::{
    RIGHT_AXIS_SECONDARY_BADGE_HEIGHT, RightAxisBadgeConnectorStyle, RightAxisBadgeKind,
    RightAxisBadgeLayout, draw_stacked_right_axis_badge, right_axis_line_end_x,
};
use super::state::ChartState;
use crate::annotations::{AnnotationKind, DrawingTool};
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme};

// ---------------------------------------------------------------------------
// Annotation Overlays
// ---------------------------------------------------------------------------

pub(super) struct AnnotationOverlayContext<'a, PriceToY>
where
    PriceToY: Fn(f64) -> f32,
{
    pub(super) frame: &'a mut canvas::Frame,
    pub(super) state: &'a ChartState,
    pub(super) theme: &'a Theme,
    pub(super) chart_w: f32,
    pub(super) chart_h: f32,
    pub(super) price_h: f32,
    pub(super) price_range: f64,
    pub(super) right_axis_badges: &'a RightAxisBadgeLayout,
    pub(super) price_to_y: &'a PriceToY,
}

impl CandlestickChart {
    pub(super) fn draw_annotation_overlays<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.price_range <= 0.0 {
            return;
        }

        for (annotation_index, ann) in self.annotations.iter().enumerate() {
            match &ann.kind {
                AnnotationKind::HorizontalLevel { price } => {
                    let y = (ctx.price_to_y)(*price);
                    if y < -10.0 || y > ctx.price_h + 10.0 {
                        continue;
                    }
                    let kind = RightAxisBadgeKind::HorizontalAnnotation(annotation_index);
                    let line_end_x =
                        right_axis_line_end_x(ctx.right_axis_badges, kind, y, ctx.chart_w);
                    stroke_segmented_hline(ctx.frame, line_end_x, y, 6.0, 4.0, ann.color, 1.0);
                    draw_stacked_right_axis_badge(
                        ctx.frame,
                        ctx.right_axis_badges,
                        kind,
                        ctx.chart_w,
                        y,
                        format_price(*price),
                        ann.color,
                        AxisBadgeStyle {
                            char_width: 6.5,
                            padding_width: 8.0,
                            height: RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
                            text_size: 9.0,
                            text_color: Color::BLACK,
                        },
                        RightAxisBadgeConnectorStyle::Segmented {
                            style: SegmentedHLineStyle {
                                segment_len: 6.0,
                                gap_len: 4.0,
                                offset: 0.0,
                                color: ann.color,
                                width: 1.0,
                            },
                        },
                    );
                }
                AnnotationKind::TrendLine { start, end } => {
                    let Some(x1) = self.timestamp_to_x(start.0, ctx.state, ctx.chart_w) else {
                        continue;
                    };
                    let y1 = (ctx.price_to_y)(start.1);
                    let Some(x2) = self.timestamp_to_x(end.0, ctx.state, ctx.chart_w) else {
                        continue;
                    };
                    let y2 = (ctx.price_to_y)(end.1);

                    let line = canvas::Path::line(Point::new(x1, y1), Point::new(x2, y2));
                    ctx.frame.stroke(
                        &line,
                        canvas::Stroke::default()
                            .with_color(ann.color)
                            .with_width(1.5),
                    );

                    for (ax, ay) in [(x1, y1), (x2, y2)] {
                        if ax >= -5.0
                            && ax <= ctx.chart_w + 5.0
                            && ay >= -5.0
                            && ay <= ctx.price_h + 5.0
                        {
                            ctx.frame.fill_rectangle(
                                Point::new(ax - 2.5, ay - 2.5),
                                Size::new(5.0, 5.0),
                                ann.color,
                            );
                        }
                    }
                }
            }
        }

        self.draw_pending_trendline_preview(ctx);
    }

    fn draw_pending_trendline_preview<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        if self.active_tool == Some(DrawingTool::TrendLine)
            && let Some((ts, price)) = ctx.state.pending_anchor
            && let Some(pos) = ctx.state.cursor_position
            && pos.x < ctx.chart_w
            && pos.y < ctx.chart_h
        {
            let Some(x1) = self.timestamp_to_x(ts, ctx.state, ctx.chart_w) else {
                return;
            };
            let y1 = (ctx.price_to_y)(price);
            let preview = canvas::Path::line(Point::new(x1, y1), Point::new(pos.x, pos.y));
            ctx.frame.stroke(
                &preview,
                canvas::Stroke::default()
                    .with_color(Color {
                        a: 0.5,
                        ..ctx.theme.palette().primary
                    })
                    .with_width(1.0),
            );
        }
    }
}
