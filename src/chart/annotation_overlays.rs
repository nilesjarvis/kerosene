use super::drawing::{
    AxisBadgeStyle, SegmentedHLineStyle, stroke_projected_segmented_hline_with_offset,
    stroke_projected_styled_line, stroke_projected_vline,
};
use super::fisheye::ChartFisheye;
use super::geometry::{LineExtension, extend_and_clip_line};
use super::model::CandlestickChart;
use super::price_badges::{
    RIGHT_AXIS_SECONDARY_BADGE_HEIGHT, RightAxisBadgeConnectorStyle, RightAxisBadgeKind,
    RightAxisBadgeLayout, draw_stacked_right_axis_badge, right_axis_line_end_x,
};
use super::state::ChartState;
use crate::annotations::{
    Anchor, Annotation, AnnotationKind, AnnotationStyle, DrawingTool, FIB_EXTENSION_LEVELS,
    FIB_RETRACEMENT_LEVELS, FibKind, LineStyle, fib_extension_price, fib_retracement_price,
};
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme, alignment};

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
    pub(super) fisheye: ChartFisheye,
    pub(super) price_to_y: &'a PriceToY,
}

/// Half-size of an anchor handle square, in pixels.
const HANDLE_HALF: f32 = 3.5;

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

        let drag_id = ctx.state.drag_annotation.as_ref().map(|ann| ann.id);
        let select_mode = self.active_tool == Some(DrawingTool::Select);
        let selected_id = if select_mode {
            ctx.state.selected_annotation
        } else {
            None
        };

        for (annotation_index, ann) in self.annotations.iter().enumerate() {
            if Some(ann.id) == drag_id {
                // Rendered from the live drag copy below at its new position.
                continue;
            }
            if !ann.style.visible {
                continue;
            }
            let selected = Some(ann.id) == selected_id;
            self.render_annotation(ctx, ann, Some(annotation_index), selected);
        }

        // In-progress drag copy is rendered at its live position and treated as
        // selected so the handles track the cursor.
        if let Some(live) = ctx.state.drag_annotation.clone() {
            self.render_annotation(ctx, &live, None, true);
        }

        self.draw_annotation_handles(ctx);
        self.draw_draft_preview(ctx);
    }

    fn render_annotation<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        ann: &Annotation,
        index: Option<usize>,
        selected: bool,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        match &ann.kind {
            AnnotationKind::HorizontalLevel { price } => {
                self.draw_horizontal_level(ctx, ann, *price, index, selected)
            }
            AnnotationKind::TrendLine { start, end } => {
                self.draw_line_annotation(ctx, ann, *start, *end, LineExtension::Segment, selected)
            }
            AnnotationKind::Ray { start, end } => {
                self.draw_line_annotation(ctx, ann, *start, *end, LineExtension::Forward, selected)
            }
            AnnotationKind::ExtendedLine { start, end } => {
                self.draw_line_annotation(ctx, ann, *start, *end, LineExtension::Both, selected)
            }
            AnnotationKind::VerticalLine { time } => {
                self.draw_vertical_line(ctx, ann, *time, selected)
            }
            AnnotationKind::Rectangle { a, b } => self.draw_rectangle(ctx, ann, *a, *b, selected),
            AnnotationKind::Measure { start, end } => {
                self.draw_measure(ctx, ann, *start, *end, selected)
            }
            AnnotationKind::Fib { kind, points } => {
                self.draw_fib(ctx, ann, *kind, points, selected)
            }
        }
    }

    // ---- Horizontal level -------------------------------------------------

    fn draw_horizontal_level<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        ann: &Annotation,
        price: f64,
        index: Option<usize>,
        selected: bool,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let y = (ctx.price_to_y)(price);
        if y < -10.0 || y > ctx.price_h + 10.0 {
            return;
        }
        if selected {
            self.stroke_halo(
                ctx,
                Point::new(0.0, y),
                Point::new(ctx.chart_w, y),
                ann.style.width,
            );
        }
        let style = SegmentedHLineStyle {
            segment_len: 6.0,
            gap_len: 4.0,
            offset: 0.0,
            color: ann.style.color,
            width: ann.style.width,
        };

        match index {
            Some(annotation_index) => {
                let kind = RightAxisBadgeKind::HorizontalAnnotation(annotation_index);
                let line_end_x = right_axis_line_end_x(ctx.right_axis_badges, kind, ctx.chart_w);
                stroke_projected_segmented_hline_with_offset(
                    ctx.frame,
                    ctx.fisheye,
                    line_end_x,
                    y,
                    style,
                );
                draw_stacked_right_axis_badge(
                    ctx.frame,
                    ctx.right_axis_badges,
                    kind,
                    ctx.chart_w,
                    y,
                    format_price(price),
                    ann.style.color,
                    AxisBadgeStyle {
                        char_width: 6.5,
                        padding_width: 8.0,
                        height: RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
                        text_size: 9.0,
                        text_color: Color::BLACK,
                    },
                    RightAxisBadgeConnectorStyle::Segmented { style },
                    ctx.fisheye,
                );
            }
            None => {
                // Drag / preview path: no stacked layout slot, draw across the
                // full width with a simple inline price tag.
                stroke_projected_segmented_hline_with_offset(
                    ctx.frame,
                    ctx.fisheye,
                    ctx.chart_w,
                    y,
                    style,
                );
                draw_label_box(
                    ctx.frame,
                    Point::new(ctx.chart_w - 64.0, y),
                    &format_price(price),
                    ann.style.color,
                    Color::BLACK,
                );
            }
        }
    }

    // ---- Lines (trend / ray / extended) -----------------------------------

    fn draw_line_annotation<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        ann: &Annotation,
        start: Anchor,
        end: Anchor,
        extension: LineExtension,
        selected: bool,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(x1) = self.timestamp_to_x(start.0, ctx.state, ctx.chart_w) else {
            return;
        };
        let Some(x2) = self.timestamp_to_x(end.0, ctx.state, ctx.chart_w) else {
            return;
        };
        let y1 = (ctx.price_to_y)(start.1);
        let y2 = (ctx.price_to_y)(end.1);
        let Some((cx1, cy1, cx2, cy2)) =
            extend_and_clip_line(x1, y1, x2, y2, ctx.chart_w, ctx.price_h, extension)
        else {
            return;
        };
        let a = Point::new(cx1, cy1);
        let b = Point::new(cx2, cy2);
        if selected {
            self.stroke_halo(ctx, a, b, ann.style.width);
        }
        stroke_projected_styled_line(
            ctx.frame,
            ctx.fisheye,
            a,
            b,
            ann.style.color,
            ann.style.width,
            ann.style.line_style,
        );
    }

    // ---- Vertical line ----------------------------------------------------

    fn draw_vertical_line<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        ann: &Annotation,
        time: u64,
        selected: bool,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(x) = self.timestamp_to_x(time, ctx.state, ctx.chart_w) else {
            return;
        };
        if x < -2.0 || x > ctx.chart_w + 2.0 {
            return;
        }
        if selected {
            self.stroke_halo(
                ctx,
                Point::new(x, 0.0),
                Point::new(x, ctx.price_h),
                ann.style.width,
            );
        }
        stroke_projected_vline(
            ctx.frame,
            ctx.fisheye,
            x,
            ctx.price_h,
            ann.style.color,
            ann.style.width,
            ann.style.line_style,
        );
    }

    // ---- Rectangle / zone -------------------------------------------------

    fn draw_rectangle<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        ann: &Annotation,
        a: Anchor,
        b: Anchor,
        selected: bool,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(xa) = self.timestamp_to_x(a.0, ctx.state, ctx.chart_w) else {
            return;
        };
        let Some(xb) = self.timestamp_to_x(b.0, ctx.state, ctx.chart_w) else {
            return;
        };
        let ya = (ctx.price_to_y)(a.1);
        let yb = (ctx.price_to_y)(b.1);
        let x0 = xa.min(xb);
        let x1 = xa.max(xb);
        let y0 = ya.min(yb);
        let y1 = ya.max(yb);

        // Clamp the fill to the visible plot rectangle.
        let fx0 = x0.max(0.0);
        let fx1 = x1.min(ctx.chart_w);
        let fy0 = y0.max(0.0);
        let fy1 = y1.min(ctx.price_h);
        if fx1 > fx0 && fy1 > fy0 {
            let fill = Color {
                a: ann.style.color.a * 0.15,
                ..ann.style.color
            };
            ctx.fisheye.fill_projected_rect(
                ctx.frame,
                Point::new(fx0, fy0),
                Size::new(fx1 - fx0, fy1 - fy0),
                fill,
            );
        }

        // Border edges (clipped individually so off-screen edges don't show).
        let edges = [
            (x0, y0, x1, y0),
            (x0, y1, x1, y1),
            (x0, y0, x0, y1),
            (x1, y0, x1, y1),
        ];
        for (ex0, ey0, ex1, ey1) in edges {
            if let Some((cx0, cy0, cx1, cy1)) = extend_and_clip_line(
                ex0,
                ey0,
                ex1,
                ey1,
                ctx.chart_w,
                ctx.price_h,
                LineExtension::Segment,
            ) {
                let a = Point::new(cx0, cy0);
                let b = Point::new(cx1, cy1);
                if selected {
                    self.stroke_halo(ctx, a, b, ann.style.width);
                }
                stroke_projected_styled_line(
                    ctx.frame,
                    ctx.fisheye,
                    a,
                    b,
                    ann.style.color,
                    ann.style.width,
                    ann.style.line_style,
                );
            }
        }
    }

    // ---- Measure ----------------------------------------------------------

    fn draw_measure<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        ann: &Annotation,
        start: Anchor,
        end: Anchor,
        selected: bool,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(x1) = self.timestamp_to_x(start.0, ctx.state, ctx.chart_w) else {
            return;
        };
        let Some(x2) = self.timestamp_to_x(end.0, ctx.state, ctx.chart_w) else {
            return;
        };
        let y1 = (ctx.price_to_y)(start.1);
        let y2 = (ctx.price_to_y)(end.1);

        // Translucent band between the two anchors.
        let fx0 = x1.min(x2).max(0.0);
        let fx1 = x1.max(x2).min(ctx.chart_w);
        let fy0 = y1.min(y2).max(0.0);
        let fy1 = y1.max(y2).min(ctx.price_h);
        if fx1 > fx0 && fy1 > fy0 {
            let up = end.1 >= start.1;
            let base = if up {
                Color::from_rgb(0.30, 0.80, 0.55)
            } else {
                Color::from_rgb(0.90, 0.40, 0.45)
            };
            ctx.fisheye.fill_projected_rect(
                ctx.frame,
                Point::new(fx0, fy0),
                Size::new(fx1 - fx0, fy1 - fy0),
                Color { a: 0.12, ..base },
            );
        }

        if let Some((cx1, cy1, cx2, cy2)) = extend_and_clip_line(
            x1,
            y1,
            x2,
            y2,
            ctx.chart_w,
            ctx.price_h,
            LineExtension::Segment,
        ) {
            let a = Point::new(cx1, cy1);
            let b = Point::new(cx2, cy2);
            if selected {
                self.stroke_halo(ctx, a, b, ann.style.width);
            }
            stroke_projected_styled_line(
                ctx.frame,
                ctx.fisheye,
                a,
                b,
                ann.style.color,
                ann.style.width,
                ann.style.line_style,
            );
        }

        // Stats label near the second anchor.
        let dprice = end.1 - start.1;
        let pct = if start.1.abs() > f64::EPSILON {
            dprice / start.1 * 100.0
        } else {
            0.0
        };
        let duration = end.0.abs_diff(start.0);
        let sign = if dprice >= 0.0 { "+" } else { "-" };
        let label = format!(
            "{sign}{} ({sign}{:.2}%) · {}",
            format_price(dprice.abs()),
            pct.abs(),
            format_compact_duration(duration),
        );
        let lx = x2.clamp(2.0, (ctx.chart_w - 4.0).max(2.0));
        let ly = y2.clamp(8.0, (ctx.price_h - 8.0).max(8.0));
        draw_label_box(
            ctx.frame,
            Point::new(lx, ly),
            &label,
            ann.style.color,
            Color::BLACK,
        );
    }

    // ---- Fibonacci --------------------------------------------------------

    fn draw_fib<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        ann: &Annotation,
        kind: FibKind,
        points: &[Anchor],
        selected: bool,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let expected = match kind {
            FibKind::Retracement => 2,
            FibKind::Extension => 3,
        };
        if points.len() != expected {
            return;
        }

        // Horizontal extent: from the leftmost anchor to the right edge.
        let mut anchor_xs = Vec::with_capacity(points.len());
        for point in points {
            let Some(x) = self.timestamp_to_x(point.0, ctx.state, ctx.chart_w) else {
                return;
            };
            anchor_xs.push(x);
        }
        let x_left = anchor_xs
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min)
            .max(0.0);
        if x_left > ctx.chart_w {
            return;
        }

        // Faint connector through the anchors.
        let connector = Color {
            a: ann.style.color.a * 0.5,
            ..ann.style.color
        };
        for window in points.windows(2) {
            let (s, e) = (window[0], window[1]);
            let (Some(sx), Some(ex)) = (
                self.timestamp_to_x(s.0, ctx.state, ctx.chart_w),
                self.timestamp_to_x(e.0, ctx.state, ctx.chart_w),
            ) else {
                continue;
            };
            let sy = (ctx.price_to_y)(s.1);
            let ey = (ctx.price_to_y)(e.1);
            if let Some((cx0, cy0, cx1, cy1)) = extend_and_clip_line(
                sx,
                sy,
                ex,
                ey,
                ctx.chart_w,
                ctx.price_h,
                LineExtension::Segment,
            ) {
                stroke_projected_styled_line(
                    ctx.frame,
                    ctx.fisheye,
                    Point::new(cx0, cy0),
                    Point::new(cx1, cy1),
                    connector,
                    ann.style.width.max(1.0),
                    LineStyle::Dotted,
                );
            }
        }

        let levels: &[f64] = match kind {
            FibKind::Retracement => FIB_RETRACEMENT_LEVELS,
            FibKind::Extension => FIB_EXTENSION_LEVELS,
        };
        for &ratio in levels {
            let price = match kind {
                FibKind::Retracement => fib_retracement_price(points[0], points[1], ratio),
                FibKind::Extension => fib_extension_price(points[0], points[1], points[2], ratio),
            };
            let y = (ctx.price_to_y)(price);
            if y < -2.0 || y > ctx.price_h + 2.0 {
                continue;
            }
            if selected {
                self.stroke_halo(
                    ctx,
                    Point::new(x_left, y),
                    Point::new(ctx.chart_w, y),
                    ann.style.width,
                );
            }
            stroke_projected_styled_line(
                ctx.frame,
                ctx.fisheye,
                Point::new(x_left, y),
                Point::new(ctx.chart_w, y),
                ann.style.color,
                ann.style.width,
                ann.style.line_style,
            );
            let label = format!("{ratio} · {}", format_price(price));
            ctx.frame.fill_text(canvas::Text {
                content: label,
                position: Point::new(x_left + 3.0, y - 1.0),
                color: ann.style.color,
                size: iced::Pixels(9.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Bottom,
                font: crate::app_fonts::monospace_font(),
                ..canvas::Text::default()
            });
        }
    }

    // ---- Selection handles ------------------------------------------------

    fn draw_annotation_handles<PriceToY>(&self, ctx: &mut AnnotationOverlayContext<'_, PriceToY>)
    where
        PriceToY: Fn(f64) -> f32,
    {
        let target = ctx.state.drag_annotation.clone().or_else(|| {
            if self.active_tool != Some(DrawingTool::Select) {
                return None;
            }
            ctx.state
                .selected_annotation
                .and_then(|id| self.annotations.iter().find(|ann| ann.id == id).cloned())
        });
        let Some(target) = target else {
            return;
        };
        let color = self.selection_color(ctx);
        for (ts, price) in target.kind.anchor_points() {
            let Some(x) = self.timestamp_to_x(ts, ctx.state, ctx.chart_w) else {
                continue;
            };
            let y = (ctx.price_to_y)(price);
            if x < -HANDLE_HALF
                || x > ctx.chart_w + HANDLE_HALF
                || y < -HANDLE_HALF
                || y > ctx.price_h + HANDLE_HALF
            {
                continue;
            }
            ctx.fisheye.fill_projected_rect(
                ctx.frame,
                Point::new(x - HANDLE_HALF, y - HANDLE_HALF),
                Size::new(HANDLE_HALF * 2.0, HANDLE_HALF * 2.0),
                color,
            );
        }
    }

    // ---- Draft preview ----------------------------------------------------

    fn draw_draft_preview<PriceToY>(&self, ctx: &mut AnnotationOverlayContext<'_, PriceToY>)
    where
        PriceToY: Fn(f64) -> f32,
    {
        let Some(tool) = self.active_tool else {
            return;
        };
        if !tool.is_shape() || ctx.state.draft_anchors.is_empty() {
            return;
        }
        let Some(cursor) = ctx.state.cursor_position else {
            return;
        };
        if cursor.x >= ctx.chart_w || cursor.y >= ctx.chart_h {
            return;
        }
        let Some((price_hi, price_range, price_h)) =
            self.visible_price_params(ctx.state, ctx.chart_w, ctx.chart_h)
        else {
            return;
        };
        let clamped_y = cursor.y.clamp(0.0, price_h);
        let price = self.y_to_price_with(clamped_y, price_hi, price_range, price_h);
        let ts = self
            .x_to_timestamp(cursor.x, ctx.state, ctx.chart_w)
            .unwrap_or(0);

        let mut anchors = ctx.state.draft_anchors.clone();
        anchors.push((ts, price));
        let Some(kind) = draft_preview_kind(tool, &anchors) else {
            return;
        };
        let base = AnnotationStyle::for_tool(tool);
        let preview = Annotation {
            id: u64::MAX,
            kind,
            style: AnnotationStyle {
                color: Color {
                    a: base.color.a * 0.6,
                    ..base.color
                },
                ..base
            },
        };
        self.render_annotation(ctx, &preview, None, false);
    }

    // ---- Shared helpers ---------------------------------------------------

    fn stroke_halo<PriceToY>(
        &self,
        ctx: &mut AnnotationOverlayContext<'_, PriceToY>,
        a: Point,
        b: Point,
        width: f32,
    ) where
        PriceToY: Fn(f64) -> f32,
    {
        let color = self.selection_color(ctx);
        ctx.fisheye.stroke_projected_line(
            ctx.frame,
            a,
            b,
            canvas::Stroke::default()
                .with_color(color)
                .with_width(width + 4.0),
        );
    }

    fn selection_color<PriceToY>(&self, ctx: &AnnotationOverlayContext<'_, PriceToY>) -> Color
    where
        PriceToY: Fn(f64) -> f32,
    {
        Color {
            a: 0.55,
            ..ctx.theme.palette().primary
        }
    }
}

/// Build the annotation kind to preview from the tool and the anchors collected
/// so far (the last anchor is the live cursor position). Returns `None` when
/// there are too few anchors to show anything meaningful.
fn draft_preview_kind(tool: DrawingTool, anchors: &[Anchor]) -> Option<AnnotationKind> {
    let two = |build: fn(Anchor, Anchor) -> AnnotationKind| -> Option<AnnotationKind> {
        (anchors.len() >= 2).then(|| build(anchors[0], anchors[1]))
    };
    match tool {
        DrawingTool::TrendLine => two(|start, end| AnnotationKind::TrendLine { start, end }),
        DrawingTool::Ray => two(|start, end| AnnotationKind::Ray { start, end }),
        DrawingTool::ExtendedLine => two(|start, end| AnnotationKind::ExtendedLine { start, end }),
        DrawingTool::Rectangle => two(|a, b| AnnotationKind::Rectangle { a, b }),
        DrawingTool::Measure => two(|start, end| AnnotationKind::Measure { start, end }),
        DrawingTool::FibRetracement => two(|a, b| AnnotationKind::Fib {
            kind: FibKind::Retracement,
            points: vec![a, b],
        }),
        DrawingTool::FibExtension => {
            if anchors.len() >= 3 {
                Some(AnnotationKind::Fib {
                    kind: FibKind::Extension,
                    points: vec![anchors[0], anchors[1], anchors[2]],
                })
            } else {
                // Preview the first leg as a plain line until the third click.
                two(|start, end| AnnotationKind::TrendLine { start, end })
            }
        }
        DrawingTool::HorizontalLevel
        | DrawingTool::VerticalLine
        | DrawingTool::Select
        | DrawingTool::Eraser => None,
    }
}

/// Draw a small filled label with text, anchored so the box hangs to the left of
/// `anchor` and is vertically centered on it. Screen-space (bypasses fisheye).
fn draw_label_box(
    frame: &mut canvas::Frame,
    anchor: Point,
    text: &str,
    background: Color,
    text_color: Color,
) {
    let width = text.len() as f32 * 6.0 + 8.0;
    let height = 14.0;
    let origin = Point::new(anchor.x - width, anchor.y - height * 0.5);
    frame.fill_rectangle(
        origin,
        Size::new(width, height),
        Color {
            a: 1.0,
            ..background
        },
    );
    frame.fill_text(canvas::Text {
        content: text.to_string(),
        position: Point::new(origin.x + 4.0, anchor.y),
        color: text_color,
        size: iced::Pixels(9.0),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..canvas::Text::default()
    });
}

/// Format a millisecond duration compactly (e.g. "3h 12m", "45m", "30s").
fn format_compact_duration(ms: u64) -> String {
    const SEC: u64 = 1_000;
    const MIN: u64 = 60 * SEC;
    const HOUR: u64 = 60 * MIN;
    const DAY: u64 = 24 * HOUR;
    if ms >= DAY {
        format!("{}d {}h", ms / DAY, (ms % DAY) / HOUR)
    } else if ms >= HOUR {
        format!("{}h {}m", ms / HOUR, (ms % HOUR) / MIN)
    } else if ms >= MIN {
        format!("{}m", ms / MIN)
    } else {
        format!("{}s", ms / SEC)
    }
}
