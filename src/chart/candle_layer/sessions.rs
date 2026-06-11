use super::CandleLayerContext;
use crate::api::Candle;
use crate::chart::drawing::{SegmentedHLineStyle, stroke_projected_segmented_hline_with_offset};
use crate::chart::fisheye::ProjectedPathPoint;
use crate::chart::model::CandlestickChart;
use crate::chart::session_indicator::{
    SessionIndicatorKind, SessionIndicatorRange, visible_session_ranges,
};
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size, Theme};

// ---------------------------------------------------------------------------
// Session Indicator Panel Rendering
// ---------------------------------------------------------------------------

const SESSION_PLOT_TOP_PADDING: f32 = 8.0;
const SESSION_PLOT_BOTTOM_PADDING: f32 = 8.0;
const SESSION_LABEL_TOP_PADDING: f32 = 4.0;
const SESSION_LABEL_MIN_WIDTH: f32 = 24.0;
const SESSION_LINE_MIN_RANGE_PCT: f64 = 0.12;
const SESSION_CHART_FILL_ALPHA: f32 = 0.042;
const SESSION_CHART_OVERNIGHT_FILL_ALPHA: f32 = 0.034;
const SESSION_BOUNDARY_LINE_ALPHA: f32 = 0.16;

impl CandlestickChart {
    pub(super) fn draw_session_chart_context<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.session_panel_h <= 0.0 {
            return;
        }

        let Some(ranges) = self.visible_session_ranges_for_view(ctx) else {
            return;
        };
        if ranges.is_empty() {
            return;
        }

        self.draw_session_chart_backgrounds(ctx, frame, &ranges);
        self.draw_session_boundary_lines(ctx, frame, &ranges, 0.0, ctx.chart_h);
    }

    pub(super) fn draw_session_panel<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.session_panel_h <= 0.0 {
            return;
        }

        let panel_y = ctx.chart_h + ctx.funding_panel_h;
        ctx.fisheye.fill_projected_rect_without_edge_blur(
            frame,
            Point::new(0.0, panel_y),
            Size::new(ctx.chart_w, ctx.session_panel_h),
            Color {
                a: 0.026,
                ..ctx.theme.palette().text
            },
        );
        let Some(ranges) = self.visible_session_ranges_for_view(ctx) else {
            return;
        };

        frame.with_clip(
            Rectangle {
                x: 0.0,
                y: panel_y,
                width: ctx.chart_w,
                height: ctx.session_panel_h,
            },
            |frame| {
                self.draw_session_backgrounds(ctx, frame, panel_y, &ranges);
                self.draw_session_boundary_lines(ctx, frame, &ranges, panel_y, ctx.session_panel_h);
                self.draw_session_return_curve(ctx, frame, panel_y, &ranges);
            },
        );

        ctx.fisheye.stroke_projected_line(
            frame,
            Point::new(0.0, panel_y),
            Point::new(ctx.chart_w, panel_y),
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.14,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
    }

    fn visible_session_ranges_for_view<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
    ) -> Option<Vec<SessionIndicatorRange>>
    where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let left_ts = self.x_to_timestamp(0.0, ctx.state, ctx.chart_w)?;
        let right_ts = self.x_to_timestamp(ctx.chart_w, ctx.state, ctx.chart_w)?;
        let visible_start = left_ts.min(right_ts);
        let visible_end = left_ts.max(right_ts);
        Some(visible_session_ranges(visible_start, visible_end))
    }

    fn draw_session_chart_backgrounds<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        ranges: &[SessionIndicatorRange],
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        for range in ranges {
            let Some((left, right)) = self.session_screen_span(ctx, *range) else {
                continue;
            };

            ctx.fisheye.fill_projected_rect_without_edge_blur(
                frame,
                Point::new(left, 0.0),
                Size::new(right - left, ctx.chart_h),
                session_chart_fill_color(range.kind, ctx.theme),
            );
        }
    }

    fn draw_session_backgrounds<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
        ranges: &[SessionIndicatorRange],
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        for range in ranges {
            let Some((left, right)) = self.session_screen_span(ctx, *range) else {
                continue;
            };
            let width = right - left;
            ctx.fisheye.fill_projected_rect_without_edge_blur(
                frame,
                Point::new(left, panel_y),
                Size::new(width, ctx.session_panel_h),
                session_fill_color(range.kind, ctx.theme),
            );
            if width >= SESSION_LABEL_MIN_WIDTH {
                frame.fill_text(canvas::Text {
                    content: range.kind.label().to_string(),
                    position: Point::new(left + 4.0, panel_y + SESSION_LABEL_TOP_PADDING),
                    color: session_label_color(range.kind, ctx.theme),
                    size: iced::Pixels(9.5),
                    align_x: alignment::Horizontal::Left.into(),
                    align_y: alignment::Vertical::Top,
                    font: crate::app_fonts::monospace_font(),
                    ..canvas::Text::default()
                });
            }
        }
    }

    fn draw_session_boundary_lines<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        ranges: &[SessionIndicatorRange],
        top_y: f32,
        height: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if height <= 0.0 {
            return;
        }

        for range in ranges {
            let Some(x) = self.timestamp_to_x(range.start_ms, ctx.state, ctx.chart_w) else {
                continue;
            };
            if x <= 0.0 || x >= ctx.chart_w {
                continue;
            }

            ctx.fisheye.stroke_projected_line_without_edge_blur(
                frame,
                Point::new(x, top_y),
                Point::new(x, top_y + height),
                canvas::Stroke::default()
                    .with_color(session_boundary_color(range.kind, ctx.theme))
                    .with_width(1.0),
            );
        }
    }

    fn session_screen_span<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        range: SessionIndicatorRange,
    ) -> Option<(f32, f32)>
    where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let start_x = self.timestamp_to_x(range.start_ms, ctx.state, ctx.chart_w)?;
        let end_x = self.timestamp_to_x(range.end_ms, ctx.state, ctx.chart_w)?;
        let left = start_x.min(end_x).clamp(0.0, ctx.chart_w);
        let right = start_x.max(end_x).clamp(0.0, ctx.chart_w);
        (right > left).then_some((left, right))
    }

    fn draw_session_return_curve<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
        ranges: &[SessionIndicatorRange],
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let plot_top = panel_y + SESSION_PLOT_TOP_PADDING;
        let plot_bottom = panel_y + ctx.session_panel_h - SESSION_PLOT_BOTTOM_PADDING;
        if plot_bottom <= plot_top {
            return;
        }

        let raw_points = self.session_return_points(ctx, ranges);
        let max_abs = raw_points
            .iter()
            .map(|point| point.return_pct.abs())
            .fold(SESSION_LINE_MIN_RANGE_PCT, f64::max);
        let baseline_y = plot_top + (plot_bottom - plot_top) * 0.5;
        let scale = (plot_bottom - plot_top) * 0.5 / max_abs as f32;

        stroke_projected_segmented_hline_with_offset(
            frame,
            ctx.fisheye,
            ctx.chart_w,
            baseline_y,
            SegmentedHLineStyle {
                segment_len: 4.0,
                gap_len: 5.0,
                offset: 0.0,
                color: Color {
                    a: 0.15,
                    ..ctx.theme.palette().text
                },
                width: 1.0,
            },
        );

        let path_points = raw_points
            .into_iter()
            .map(|point| ProjectedPathPoint {
                point: Point::new(
                    point.x,
                    baseline_y
                        - (point.return_pct as f32 * scale)
                            .clamp(plot_top - baseline_y, plot_bottom - baseline_y),
                ),
                starts_segment: point.starts_segment,
            })
            .collect::<Vec<_>>();
        if path_points.len() < 2 {
            return;
        }

        ctx.fisheye.stroke_projected_path_points(
            frame,
            &path_points,
            canvas::Stroke::default()
                .with_color(session_curve_color(ctx.theme))
                .with_width(1.55),
        );
    }

    fn session_return_points<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        ranges: &[SessionIndicatorRange],
    ) -> Vec<SessionReturnPoint>
    where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let mut points = Vec::new();
        for range in ranges {
            let Some((base_idx, base_open)) = session_base_candle(&self.candles, *range) else {
                continue;
            };
            let start_idx = self
                .candles
                .partition_point(|candle| candle.open_time < range.start_ms)
                .max(base_idx)
                .max(ctx.first_vis);
            let end_idx = self
                .candles
                .partition_point(|candle| candle.open_time < range.end_ms)
                .min(ctx.last_vis.saturating_add(1));
            if start_idx >= end_idx {
                continue;
            }
            let mut starts_segment = true;
            if let Some(x) = self.timestamp_to_x(range.start_ms, ctx.state, ctx.chart_w)
                && x >= -ctx.step
                && x <= ctx.chart_w + ctx.step
            {
                points.push(SessionReturnPoint {
                    x,
                    return_pct: 0.0,
                    starts_segment,
                });
                starts_segment = false;
            }
            for idx in start_idx..end_idx {
                let candle = &self.candles[idx];
                if !candle.close.is_finite() {
                    continue;
                }
                let x = (ctx.idx_to_cx)(idx);
                if x < -ctx.step || x > ctx.chart_w + ctx.step {
                    continue;
                }
                let return_pct = ((candle.close - base_open) / base_open) * 100.0;
                if !return_pct.is_finite() {
                    continue;
                }
                points.push(SessionReturnPoint {
                    x,
                    return_pct,
                    starts_segment,
                });
                starts_segment = false;
            }
        }
        points
    }
}

#[derive(Debug, Clone, Copy)]
struct SessionReturnPoint {
    x: f32,
    return_pct: f64,
    starts_segment: bool,
}

fn session_base_candle(candles: &[Candle], range: SessionIndicatorRange) -> Option<(usize, f64)> {
    let start = candles.partition_point(|candle| candle.open_time < range.start_ms);
    candles
        .iter()
        .enumerate()
        .skip(start)
        .take_while(|(_, candle)| candle.open_time < range.end_ms)
        .find_map(|(idx, candle)| {
            (candle.open.is_finite() && candle.open > 0.0).then_some((idx, candle.open))
        })
}

fn session_base_color(kind: SessionIndicatorKind, theme: &Theme) -> Color {
    let palette = theme.extended_palette();
    match kind {
        SessionIndicatorKind::NewYork => mix_color(
            mix_color(
                palette.primary.base.color,
                palette.primary.strong.color,
                0.72,
            ),
            palette.background.base.text,
            0.12,
        ),
        SessionIndicatorKind::Asia => mix_color(
            palette.warning.base.color,
            palette.warning.strong.color,
            0.32,
        ),
        SessionIndicatorKind::London => mix_color(
            mix_color(
                palette.success.base.color,
                palette.success.strong.color,
                0.90,
            ),
            palette.background.base.color,
            0.16,
        ),
        SessionIndicatorKind::Overnight => mix_color(
            palette.secondary.base.color,
            palette.secondary.weak.color,
            0.42,
        ),
    }
}

fn session_fill_color(kind: SessionIndicatorKind, theme: &Theme) -> Color {
    let mut color = session_base_color(kind, theme);
    color.a = match kind {
        SessionIndicatorKind::Overnight => 0.115,
        _ => 0.135,
    };
    color
}

fn session_chart_fill_color(kind: SessionIndicatorKind, theme: &Theme) -> Color {
    let mut color = session_base_color(kind, theme);
    color.a = match kind {
        SessionIndicatorKind::Overnight => SESSION_CHART_OVERNIGHT_FILL_ALPHA,
        _ => SESSION_CHART_FILL_ALPHA,
    };
    color
}

fn session_label_color(kind: SessionIndicatorKind, theme: &Theme) -> Color {
    let mut color = session_base_color(kind, theme);
    color.a = 0.82;
    color
}

fn session_boundary_color(kind: SessionIndicatorKind, theme: &Theme) -> Color {
    let mut color = session_label_color(kind, theme);
    color.a = SESSION_BOUNDARY_LINE_ALPHA;
    color
}

fn mix_color(a: Color, b: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r + (b.r - a.r) * factor,
        a.g + (b.g - a.g) * factor,
        a.b + (b.b - a.b) * factor,
        a.a + (b.a - a.a) * factor,
    )
}

fn session_curve_color(theme: &Theme) -> Color {
    Color {
        a: 0.92,
        ..theme.extended_palette().warning.strong.color
    }
}

#[cfg(test)]
mod tests {
    use super::{session_base_candle, session_base_color};
    use crate::api::Candle;
    use crate::chart::session_indicator::{SessionIndicatorKind, SessionIndicatorRange};
    use iced::Theme;

    fn candle(open_time: u64, open: f64, close: f64) -> Candle {
        Candle::test_ohlcv(
            open_time,
            open_time + 60_000,
            [open, open, open, close],
            1.0,
        )
    }

    #[test]
    fn session_base_open_uses_first_valid_open_inside_range() {
        let range = SessionIndicatorRange {
            kind: SessionIndicatorKind::Asia,
            start_ms: 1_000,
            end_ms: 5_000,
        };
        let candles = vec![
            candle(0, 99.0, 99.0),
            candle(1_000, 0.0, 100.0),
            candle(2_000, 101.0, 102.0),
            candle(6_000, 200.0, 201.0),
        ];

        assert_eq!(session_base_candle(&candles, range), Some((2, 101.0)));
    }

    #[test]
    fn session_base_open_skips_empty_ranges() {
        let range = SessionIndicatorRange {
            kind: SessionIndicatorKind::London,
            start_ms: 10_000,
            end_ms: 11_000,
        };
        assert_eq!(
            session_base_candle(&[candle(1_000, 100.0, 101.0)], range),
            None
        );
    }

    #[test]
    fn session_colors_are_distinct() {
        for theme in Theme::ALL {
            let colors = [
                session_base_color(SessionIndicatorKind::NewYork, theme),
                session_base_color(SessionIndicatorKind::Asia, theme),
                session_base_color(SessionIndicatorKind::London, theme),
                session_base_color(SessionIndicatorKind::Overnight, theme),
            ];

            for (left_idx, left) in colors.iter().enumerate() {
                for right in colors.iter().skip(left_idx + 1) {
                    let distance = color_distance(*left, *right);
                    assert!(
                        distance >= 0.08,
                        "theme {theme:?} session colors are too close: {distance:.3}"
                    );
                }
            }
        }
    }

    fn color_distance(left: iced::Color, right: iced::Color) -> f32 {
        let dr = left.r - right.r;
        let dg = left.g - right.g;
        let db = left.b - right.b;
        (dr * dr + dg * dg + db * db).sqrt()
    }
}
