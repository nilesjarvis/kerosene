use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Chart Fisheye Projection
// ---------------------------------------------------------------------------

const MAX_BARREL_COEFFICIENT: f32 = 0.18;
const MAX_CHROMATIC_SHIFT_PX: f32 = 7.0;
const MAX_EDGE_BLUR_SHIFT_PX: f32 = 4.8;
const CHROMATIC_STROKE_ALPHA: f32 = 0.55;
const CHROMATIC_FILL_ALPHA: f32 = 0.34;
const CHROMATIC_ALPHA_CEILING: f32 = 0.6;
const EDGE_BLUR_STROKE_ALPHA: f32 = 0.26;
const EDGE_BLUR_FILL_ALPHA: f32 = 0.16;
const EDGE_BLUR_ALPHA_CEILING: f32 = 0.28;
const LINE_SAMPLE_PX: f32 = 18.0;
const MAX_LINE_SAMPLES: usize = 96;
const NEWTON_STEPS: usize = 6;
const EDGE_BLUR_BUCKETS: usize = 6;
const EDGE_BLUR_INNER_RADIUS: f32 = 0.22;
const EDGE_BLUR_RADIUS_RANGE: f32 = 1.0 - EDGE_BLUR_INNER_RADIUS;
// The blur is a soft-focus skirt: geometry is re-stroked in place with these
// widened, fading layers rather than displaced ghost copies, which read as
// double vision instead of defocus.
const EDGE_BLUR_LAYERS: [EdgeBlurLayer; 2] = [
    EdgeBlurLayer {
        radius_scale: 0.55,
        alpha_scale: 1.0,
    },
    EdgeBlurLayer {
        radius_scale: 1.0,
        alpha_scale: 0.5,
    },
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ChartFisheye {
    enabled: bool,
    strength: f32,
    chromatic_enabled: bool,
    chromatic_strength: f32,
    edge_blur_enabled: bool,
    edge_blur_strength: f32,
    width: f32,
    height: f32,
}

impl ChartFisheye {
    pub(crate) fn new(enabled: bool, strength: f32, width: f32, height: f32) -> Self {
        let enabled = enabled
            && strength.is_finite()
            && strength > 0.0
            && width > 0.0
            && height > 0.0
            && width.is_finite()
            && height.is_finite();

        Self {
            enabled,
            strength: if strength.is_finite() {
                strength.clamp(0.0, 1.0)
            } else {
                0.0
            },
            chromatic_enabled: false,
            chromatic_strength: 0.0,
            edge_blur_enabled: false,
            edge_blur_strength: 0.0,
            width,
            height,
        }
    }

    pub(crate) fn with_chromatic(mut self, enabled: bool, strength: f32) -> Self {
        self.chromatic_enabled = enabled
            && strength.is_finite()
            && strength > 0.0
            && self.width > 0.0
            && self.height > 0.0
            && self.width.is_finite()
            && self.height.is_finite();
        self.chromatic_strength = if strength.is_finite() {
            strength.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }

    pub(crate) fn with_edge_blur(mut self, enabled: bool, strength: f32) -> Self {
        self.edge_blur_enabled = enabled
            && strength.is_finite()
            && strength > 0.0
            && self.width > 0.0
            && self.height > 0.0
            && self.width.is_finite()
            && self.height.is_finite();
        self.edge_blur_strength = if strength.is_finite() {
            strength.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }

    pub(crate) fn disabled() -> Self {
        Self {
            enabled: false,
            strength: 0.0,
            chromatic_enabled: false,
            chromatic_strength: 0.0,
            edge_blur_enabled: false,
            edge_blur_strength: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub(crate) fn is_enabled(self) -> bool {
        self.enabled || self.chromatic_enabled || self.edge_blur_enabled
    }

    pub(crate) fn distorts_geometry(self) -> bool {
        self.enabled
    }

    pub(crate) fn project(self, point: Point) -> Point {
        if !self.enabled || !self.contains_projection_y(point.y) || !point.x.is_finite() {
            return point;
        }

        let (nx, ny) = self.to_normalized(point);
        let radius_sq = nx * nx + ny * ny;
        let scale = 1.0 + self.coefficient() * radius_sq;
        self.clamp_projected_point(self.denormalize_point(nx * scale, ny * scale))
    }

    pub(crate) fn unproject(self, point: Point) -> Point {
        if !self.enabled || !self.contains_point(point) {
            return point;
        }

        let (px, py) = self.to_normalized(point);
        let projected_radius = (px * px + py * py).sqrt();
        if projected_radius <= f32::EPSILON {
            return self.denormalize_point(0.0, 0.0);
        }

        let coefficient = self.coefficient();
        let mut radius =
            projected_radius / (1.0 + coefficient * projected_radius * projected_radius);
        for _ in 0..NEWTON_STEPS {
            let f = radius + coefficient * radius * radius * radius - projected_radius;
            let df = 1.0 + 3.0 * coefficient * radius * radius;
            if df.abs() <= f32::EPSILON {
                break;
            }
            radius -= f / df;
        }

        let direction_scale = radius / projected_radius;
        self.denormalize_point(px * direction_scale, py * direction_scale)
    }

    pub(crate) fn stroke_projected_line<'a>(
        self,
        frame: &mut canvas::Frame,
        start: Point,
        end: Point,
        stroke: canvas::Stroke<'a>,
    ) {
        if !self.is_enabled() {
            let line = canvas::Path::line(start, end);
            frame.stroke(&line, stroke);
            return;
        }

        if !valid_point(start) || !valid_point(end) {
            return;
        }

        let samples = line_samples(start, end);
        if self.edge_blur_enabled {
            self.stroke_edge_blurred_line(frame, start, end, samples, stroke);
        }
        if self.chromatic_enabled {
            self.stroke_chromatic_line(frame, start, end, samples, stroke);
        }
        let path = self.line_path(start, end, samples, ChromaticChannel::Main);
        frame.stroke(&path, stroke);
    }

    pub(crate) fn stroke_projected_line_without_edge_blur<'a>(
        self,
        frame: &mut canvas::Frame,
        start: Point,
        end: Point,
        stroke: canvas::Stroke<'a>,
    ) {
        self.without_edge_blur()
            .stroke_projected_line(frame, start, end, stroke);
    }

    pub(crate) fn fill_projected_rect(
        self,
        frame: &mut canvas::Frame,
        origin: Point,
        size: Size,
        color: Color,
    ) {
        if !self.is_enabled() {
            frame.fill_rectangle(origin, size, color);
            return;
        }

        if !valid_point(origin)
            || size.width <= 0.0
            || size.height <= 0.0
            || !size.width.is_finite()
            || !size.height.is_finite()
        {
            return;
        }

        let cols = rect_subdivisions(size.width);
        let rows = rect_subdivisions(size.height);
        if self.edge_blur_enabled {
            self.fill_edge_blurred_rect(frame, origin, size, cols, rows, color);
        }
        if self.chromatic_enabled {
            let red_path = canvas::Path::new(|path| {
                self.append_rect_for_channel(path, origin, size, cols, rows, ChromaticChannel::Red);
            });
            let cyan_path = canvas::Path::new(|path| {
                self.append_rect_for_channel(
                    path,
                    origin,
                    size,
                    cols,
                    rows,
                    ChromaticChannel::Cyan,
                );
            });
            frame.fill(
                &red_path,
                self.chromatic_color(color, ChromaticChannel::Red, CHROMATIC_FILL_ALPHA),
            );
            frame.fill(
                &cyan_path,
                self.chromatic_color(color, ChromaticChannel::Cyan, CHROMATIC_FILL_ALPHA),
            );
        }

        let path = canvas::Path::new(|path| {
            self.append_projected_rect(path, origin, size, cols, rows);
        });
        frame.fill(&path, color);
    }

    pub(crate) fn fill_projected_rect_without_edge_blur(
        self,
        frame: &mut canvas::Frame,
        origin: Point,
        size: Size,
        color: Color,
    ) {
        self.without_edge_blur()
            .fill_projected_rect(frame, origin, size, color);
    }

    pub(crate) fn stroke_projected_rect_without_edge_blur<'a>(
        self,
        frame: &mut canvas::Frame,
        origin: Point,
        size: Size,
        stroke: canvas::Stroke<'a>,
    ) {
        self.without_edge_blur()
            .stroke_projected_rect(frame, origin, size, stroke);
    }

    pub(crate) fn stroke_projected_circle_without_edge_blur<'a>(
        self,
        frame: &mut canvas::Frame,
        center: Point,
        radius: f32,
        stroke: canvas::Stroke<'a>,
    ) {
        self.without_edge_blur()
            .stroke_projected_circle(frame, center, radius, stroke);
    }

    pub(crate) fn fill_projected_circle_without_edge_blur(
        self,
        frame: &mut canvas::Frame,
        center: Point,
        radius: f32,
        color: Color,
    ) {
        self.without_edge_blur()
            .fill_projected_circle(frame, center, radius, color);
    }

    pub(crate) fn fill_projected_polygon_without_edge_blur(
        self,
        frame: &mut canvas::Frame,
        points: &[Point],
        color: Color,
    ) {
        self.without_edge_blur()
            .fill_projected_polygon(frame, points, color);
    }

    pub(crate) fn fill_projected_rects(
        self,
        frame: &mut canvas::Frame,
        rects: &[(Point, Size)],
        color: Color,
    ) {
        if rects.is_empty() {
            return;
        }

        if !self.is_enabled() {
            let path = canvas::Path::new(|path| {
                for (origin, size) in rects {
                    if valid_rect(*origin, *size) {
                        path.rectangle(*origin, *size);
                    }
                }
            });
            frame.fill(&path, color);
            return;
        }

        if self.edge_blur_enabled {
            self.fill_edge_blurred_rects(frame, rects, color);
        }

        if self.chromatic_enabled {
            let red_path = canvas::Path::new(|path| {
                for (origin, size) in rects {
                    if !valid_rect(*origin, *size) {
                        continue;
                    }
                    self.append_rect_for_channel(
                        path,
                        *origin,
                        *size,
                        rect_subdivisions(size.width),
                        rect_subdivisions(size.height),
                        ChromaticChannel::Red,
                    );
                }
            });
            let cyan_path = canvas::Path::new(|path| {
                for (origin, size) in rects {
                    if !valid_rect(*origin, *size) {
                        continue;
                    }
                    self.append_rect_for_channel(
                        path,
                        *origin,
                        *size,
                        rect_subdivisions(size.width),
                        rect_subdivisions(size.height),
                        ChromaticChannel::Cyan,
                    );
                }
            });
            frame.fill(
                &red_path,
                self.chromatic_color(color, ChromaticChannel::Red, CHROMATIC_FILL_ALPHA),
            );
            frame.fill(
                &cyan_path,
                self.chromatic_color(color, ChromaticChannel::Cyan, CHROMATIC_FILL_ALPHA),
            );
        }

        let path = canvas::Path::new(|path| {
            for (origin, size) in rects {
                if !valid_rect(*origin, *size) {
                    continue;
                }
                self.append_projected_rect(
                    path,
                    *origin,
                    *size,
                    rect_subdivisions(size.width),
                    rect_subdivisions(size.height),
                );
            }
        });
        frame.fill(&path, color);
    }

    pub(crate) fn fill_projected_micro_rects(
        self,
        frame: &mut canvas::Frame,
        rects: &[(Point, Size)],
        color: Color,
    ) {
        if !self.edge_blur_enabled {
            self.fill_projected_rects(frame, rects, color);
            return;
        }

        if rects.is_empty() {
            return;
        }

        let blur_buckets = self.edge_blur_rect_buckets(rects);
        self.fill_edge_blurred_rect_buckets(
            frame,
            &blur_buckets,
            color,
            EDGE_BLUR_FILL_ALPHA * 0.65,
        );

        if self.chromatic_enabled {
            let red_path = canvas::Path::new(|path| {
                for (origin, size) in rects {
                    if !valid_rect(*origin, *size) {
                        continue;
                    }
                    self.append_rect_for_channel(
                        path,
                        *origin,
                        *size,
                        rect_subdivisions(size.width),
                        rect_subdivisions(size.height),
                        ChromaticChannel::Red,
                    );
                }
            });
            let cyan_path = canvas::Path::new(|path| {
                for (origin, size) in rects {
                    if !valid_rect(*origin, *size) {
                        continue;
                    }
                    self.append_rect_for_channel(
                        path,
                        *origin,
                        *size,
                        rect_subdivisions(size.width),
                        rect_subdivisions(size.height),
                        ChromaticChannel::Cyan,
                    );
                }
            });
            frame.fill(
                &red_path,
                self.chromatic_color(color, ChromaticChannel::Red, CHROMATIC_FILL_ALPHA),
            );
            frame.fill(
                &cyan_path,
                self.chromatic_color(color, ChromaticChannel::Cyan, CHROMATIC_FILL_ALPHA),
            );
        }

        let path = canvas::Path::new(|path| {
            for (origin, size) in rects {
                if !valid_rect(*origin, *size) {
                    continue;
                }
                self.append_projected_rect(
                    path,
                    *origin,
                    *size,
                    rect_subdivisions(size.width),
                    rect_subdivisions(size.height),
                );
            }
        });
        frame.fill(&path, color);
    }

    pub(crate) fn fill_projected_rect_flat(
        self,
        frame: &mut canvas::Frame,
        origin: Point,
        size: Size,
        color: Color,
    ) {
        if !self.enabled {
            frame.fill_rectangle(origin, size, color);
            return;
        }

        if !valid_rect(origin, size) {
            return;
        }

        let path = canvas::Path::new(|path| {
            self.append_projected_rect(path, origin, size, 1, 1);
        });
        frame.fill(&path, color);
    }

    pub(crate) fn stroke_projected_rect<'a>(
        self,
        frame: &mut canvas::Frame,
        origin: Point,
        size: Size,
        stroke: canvas::Stroke<'a>,
    ) {
        if !self.is_enabled() {
            let rect = canvas::Path::rectangle(origin, size);
            frame.stroke(&rect, stroke);
            return;
        }

        let p1 = origin;
        let p2 = Point::new(origin.x + size.width, origin.y);
        let p3 = Point::new(origin.x + size.width, origin.y + size.height);
        let p4 = Point::new(origin.x, origin.y + size.height);
        self.stroke_projected_line(frame, p1, p2, stroke);
        self.stroke_projected_line(frame, p2, p3, stroke);
        self.stroke_projected_line(frame, p3, p4, stroke);
        self.stroke_projected_line(frame, p4, p1, stroke);
    }

    pub(crate) fn stroke_projected_circle<'a>(
        self,
        frame: &mut canvas::Frame,
        center: Point,
        radius: f32,
        stroke: canvas::Stroke<'a>,
    ) {
        if !self.is_enabled() {
            let circle = canvas::Path::circle(center, radius);
            frame.stroke(&circle, stroke);
            return;
        }
        if radius <= 0.0 || !radius.is_finite() || !valid_point(center) {
            return;
        }

        let samples = 48;
        if self.edge_blur_enabled {
            self.stroke_edge_blurred_circle(frame, center, radius, samples, stroke);
        }
        if self.chromatic_enabled {
            let red_path = self.circle_path(center, radius, samples, ChromaticChannel::Red);
            let cyan_path = self.circle_path(center, radius, samples, ChromaticChannel::Cyan);
            let source_color = stroke_color(&stroke);
            frame.stroke(
                &red_path,
                stroke.with_color(self.chromatic_color(
                    source_color,
                    ChromaticChannel::Red,
                    CHROMATIC_STROKE_ALPHA,
                )),
            );
            frame.stroke(
                &cyan_path,
                stroke.with_color(self.chromatic_color(
                    source_color,
                    ChromaticChannel::Cyan,
                    CHROMATIC_STROKE_ALPHA,
                )),
            );
        }
        let path = self.circle_path(center, radius, samples, ChromaticChannel::Main);
        frame.stroke(&path, stroke);
    }

    pub(crate) fn fill_projected_circle(
        self,
        frame: &mut canvas::Frame,
        center: Point,
        radius: f32,
        color: Color,
    ) {
        if !self.is_enabled() {
            let circle = canvas::Path::circle(center, radius);
            frame.fill(&circle, color);
            return;
        }
        if radius <= 0.0 || !radius.is_finite() || !valid_point(center) {
            return;
        }

        let samples = 32;
        if self.edge_blur_enabled {
            self.fill_edge_blurred_circle(frame, center, radius, samples, color);
        }
        if self.chromatic_enabled {
            let red_path = self.circle_path(center, radius, samples, ChromaticChannel::Red);
            let cyan_path = self.circle_path(center, radius, samples, ChromaticChannel::Cyan);
            frame.fill(
                &red_path,
                self.chromatic_color(color, ChromaticChannel::Red, CHROMATIC_FILL_ALPHA),
            );
            frame.fill(
                &cyan_path,
                self.chromatic_color(color, ChromaticChannel::Cyan, CHROMATIC_FILL_ALPHA),
            );
        }
        let path = self.circle_path(center, radius, samples, ChromaticChannel::Main);
        frame.fill(&path, color);
    }

    pub(crate) fn fill_projected_polygon(
        self,
        frame: &mut canvas::Frame,
        points: &[Point],
        color: Color,
    ) {
        if points.len() < 3 || points.iter().any(|point| !valid_point(*point)) {
            return;
        }

        if self.edge_blur_enabled {
            self.fill_edge_blurred_polygon(frame, points, color);
        }

        if self.chromatic_enabled {
            let red_path = self.polygon_path(points, ChromaticChannel::Red);
            let cyan_path = self.polygon_path(points, ChromaticChannel::Cyan);
            frame.fill(
                &red_path,
                self.chromatic_color(color, ChromaticChannel::Red, CHROMATIC_FILL_ALPHA),
            );
            frame.fill(
                &cyan_path,
                self.chromatic_color(color, ChromaticChannel::Cyan, CHROMATIC_FILL_ALPHA),
            );
        }

        let path = self.polygon_path(points, ChromaticChannel::Main);
        frame.fill(&path, color);
    }

    pub(crate) fn stroke_projected_path_points<'a>(
        self,
        frame: &mut canvas::Frame,
        points: &[ProjectedPathPoint],
        stroke: canvas::Stroke<'a>,
    ) {
        if self.edge_blur_enabled {
            self.stroke_edge_blurred_path_points(frame, points, stroke);
        }

        if self.chromatic_enabled {
            let red_path = self.path_points(points, ChromaticChannel::Red);
            let cyan_path = self.path_points(points, ChromaticChannel::Cyan);
            let source_color = stroke_color(&stroke);
            frame.stroke(
                &red_path,
                stroke.with_color(self.chromatic_color(
                    source_color,
                    ChromaticChannel::Red,
                    CHROMATIC_STROKE_ALPHA,
                )),
            );
            frame.stroke(
                &cyan_path,
                stroke.with_color(self.chromatic_color(
                    source_color,
                    ChromaticChannel::Cyan,
                    CHROMATIC_STROKE_ALPHA,
                )),
            );
        }

        let path = self.path_points(points, ChromaticChannel::Main);
        frame.stroke(&path, stroke);
    }

    pub(crate) fn append_projected_rect(
        self,
        path: &mut canvas::path::Builder,
        origin: Point,
        size: Size,
        cols: usize,
        rows: usize,
    ) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let cell_w = size.width / cols as f32;
        let cell_h = size.height / rows as f32;

        for row in 0..rows {
            for col in 0..cols {
                let left = origin.x + col as f32 * cell_w;
                let top = origin.y + row as f32 * cell_h;
                let right = if col + 1 == cols {
                    origin.x + size.width
                } else {
                    left + cell_w
                };
                let bottom = if row + 1 == rows {
                    origin.y + size.height
                } else {
                    top + cell_h
                };

                path.move_to(self.project(Point::new(left, top)));
                path.line_to(self.project(Point::new(right, top)));
                path.line_to(self.project(Point::new(right, bottom)));
                path.line_to(self.project(Point::new(left, bottom)));
                path.close();
            }
        }
    }

    fn append_rect_for_channel(
        self,
        path: &mut canvas::path::Builder,
        origin: Point,
        size: Size,
        cols: usize,
        rows: usize,
        channel: ChromaticChannel,
    ) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let cell_w = size.width / cols as f32;
        let cell_h = size.height / rows as f32;

        for row in 0..rows {
            for col in 0..cols {
                let left = origin.x + col as f32 * cell_w;
                let top = origin.y + row as f32 * cell_h;
                let right = if col + 1 == cols {
                    origin.x + size.width
                } else {
                    left + cell_w
                };
                let bottom = if row + 1 == rows {
                    origin.y + size.height
                } else {
                    top + cell_h
                };

                path.move_to(self.visual_point(Point::new(left, top), channel));
                path.line_to(self.visual_point(Point::new(right, top), channel));
                path.line_to(self.visual_point(Point::new(right, bottom), channel));
                path.line_to(self.visual_point(Point::new(left, bottom), channel));
                path.close();
            }
        }
    }

    fn append_projected_rect_boundary(
        self,
        path: &mut canvas::path::Builder,
        origin: Point,
        size: Size,
        cols: usize,
        rows: usize,
    ) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let right = origin.x + size.width;
        let bottom = origin.y + size.height;
        let cell_w = size.width / cols as f32;
        let cell_h = size.height / rows as f32;

        path.move_to(self.project(origin));
        for col in 1..=cols {
            let x = if col == cols {
                right
            } else {
                origin.x + col as f32 * cell_w
            };
            path.line_to(self.project(Point::new(x, origin.y)));
        }
        for row in 1..=rows {
            let y = if row == rows {
                bottom
            } else {
                origin.y + row as f32 * cell_h
            };
            path.line_to(self.project(Point::new(right, y)));
        }
        for col in (0..cols).rev() {
            let x = if col == 0 {
                origin.x
            } else {
                origin.x + col as f32 * cell_w
            };
            path.line_to(self.project(Point::new(x, bottom)));
        }
        for row in (1..rows).rev() {
            path.line_to(self.project(Point::new(origin.x, origin.y + row as f32 * cell_h)));
        }
        path.close();
    }

    fn stroke_edge_blurred_line<'a>(
        self,
        frame: &mut canvas::Frame,
        start: Point,
        end: Point,
        samples: usize,
        stroke: canvas::Stroke<'a>,
    ) {
        let mut points = Vec::with_capacity(samples + 1);
        points.push(ProjectedPathPoint {
            point: self.project(start),
            starts_segment: true,
        });
        for sample in 1..=samples {
            let t = sample as f32 / samples as f32;
            points.push(ProjectedPathPoint {
                point: self.project(lerp_point(start, end, t)),
                starts_segment: false,
            });
        }
        self.stroke_edge_blurred_polyline(frame, &points, stroke);
    }

    /// Stroke soft skirts under an already-projected polyline, bucketing
    /// sub-segments by their local edge factor so the blur tapers along the
    /// geometry instead of hazing its full length at the edge maximum.
    fn stroke_edge_blurred_polyline<'a>(
        self,
        frame: &mut canvas::Frame,
        points: &[ProjectedPathPoint],
        stroke: canvas::Stroke<'a>,
    ) {
        let source_color = stroke_color(&stroke);
        let runs = self.edge_blur_polyline_runs(points);
        for (bucket, bucket_runs) in runs.iter().enumerate() {
            if bucket_runs.is_empty() {
                continue;
            }
            let factor = edge_blur_bucket_factor(bucket);
            let radius = self.edge_blur_strength * MAX_EDGE_BLUR_SHIFT_PX * factor;
            if radius <= f32::EPSILON {
                continue;
            }
            let path = path_from_runs(bucket_runs);
            for layer in EDGE_BLUR_LAYERS {
                frame.stroke(
                    &path,
                    stroke
                        .with_color(self.edge_blur_color(
                            source_color,
                            EDGE_BLUR_STROKE_ALPHA * layer.alpha_scale,
                            factor,
                        ))
                        .with_width(stroke.width + 2.0 * radius * layer.radius_scale),
                );
            }
        }
    }

    fn edge_blur_polyline_runs(
        self,
        points: &[ProjectedPathPoint],
    ) -> [Vec<Vec<Point>>; EDGE_BLUR_BUCKETS] {
        let mut runs: [Vec<Vec<Point>>; EDGE_BLUR_BUCKETS] = std::array::from_fn(|_| Vec::new());
        let mut previous: Option<(Point, usize)> = None;

        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b.starts_segment {
                previous = None;
                continue;
            }
            let factor = self
                .edge_blur_factor(a.point)
                .max(self.edge_blur_factor(b.point));
            let Some(bucket) = edge_blur_bucket(factor) else {
                previous = None;
                continue;
            };
            let extends_previous = matches!(previous, Some((last, last_bucket)) if last_bucket == bucket && last == a.point);
            if extends_previous {
                if let Some(run) = runs[bucket].last_mut() {
                    run.push(b.point);
                }
            } else {
                runs[bucket].push(vec![a.point, b.point]);
            }
            previous = Some((b.point, bucket));
        }
        runs
    }

    /// Stroke a filled shape's boundary with widened, fading layers of its own
    /// color to soften the silhouette; the inner half of each skirt hides
    /// under the fill drawn on top.
    fn stroke_soft_boundary(
        self,
        frame: &mut canvas::Frame,
        boundary: &canvas::Path,
        color: Color,
        factor: f32,
        alpha_base: f32,
    ) {
        let radius = self.edge_blur_strength * MAX_EDGE_BLUR_SHIFT_PX * factor;
        if radius <= f32::EPSILON {
            return;
        }
        for layer in EDGE_BLUR_LAYERS {
            frame.stroke(
                boundary,
                canvas::Stroke::default()
                    .with_color(self.edge_blur_color(color, alpha_base * layer.alpha_scale, factor))
                    .with_width(2.0 * radius * layer.radius_scale),
            );
        }
    }

    fn fill_edge_blurred_rect(
        self,
        frame: &mut canvas::Frame,
        origin: Point,
        size: Size,
        cols: usize,
        rows: usize,
        color: Color,
    ) {
        let blur_factor = self.edge_blur_factor_for_rect(origin, size);
        if blur_factor <= f32::EPSILON {
            return;
        }

        let boundary = canvas::Path::new(|path| {
            self.append_projected_rect_boundary(path, origin, size, cols, rows);
        });
        self.stroke_soft_boundary(frame, &boundary, color, blur_factor, EDGE_BLUR_FILL_ALPHA);
    }

    fn fill_edge_blurred_rects(
        self,
        frame: &mut canvas::Frame,
        rects: &[(Point, Size)],
        color: Color,
    ) {
        let blur_buckets = self.edge_blur_rect_buckets(rects);
        self.fill_edge_blurred_rect_buckets(frame, &blur_buckets, color, EDGE_BLUR_FILL_ALPHA);
    }

    fn edge_blur_rect_buckets(
        self,
        rects: &[(Point, Size)],
    ) -> [Vec<EdgeBlurRect>; EDGE_BLUR_BUCKETS] {
        let mut buckets = std::array::from_fn(|_| Vec::new());
        for (origin, size) in rects {
            if !valid_rect(*origin, *size) {
                continue;
            }
            let blur_factor = self.edge_blur_factor_for_rect(*origin, *size);
            let Some(bucket) = edge_blur_bucket(blur_factor) else {
                continue;
            };
            buckets[bucket].push(EdgeBlurRect {
                origin: *origin,
                size: *size,
                cols: rect_subdivisions(size.width),
                rows: rect_subdivisions(size.height),
            });
        }
        buckets
    }

    fn fill_edge_blurred_rect_buckets(
        self,
        frame: &mut canvas::Frame,
        buckets: &[Vec<EdgeBlurRect>; EDGE_BLUR_BUCKETS],
        color: Color,
        alpha_scale: f32,
    ) {
        for (bucket, rects) in buckets.iter().enumerate() {
            if rects.is_empty() {
                continue;
            }

            let bucket_factor = edge_blur_bucket_factor(bucket);
            let boundary = canvas::Path::new(|path| {
                for rect in rects {
                    self.append_projected_rect_boundary(
                        path,
                        rect.origin,
                        rect.size,
                        rect.cols,
                        rect.rows,
                    );
                }
            });
            self.stroke_soft_boundary(frame, &boundary, color, bucket_factor, alpha_scale);
        }
    }

    fn stroke_edge_blurred_circle<'a>(
        self,
        frame: &mut canvas::Frame,
        center: Point,
        radius: f32,
        samples: usize,
        stroke: canvas::Stroke<'a>,
    ) {
        let mut points = Vec::with_capacity(samples + 1);
        points.push(ProjectedPathPoint {
            point: self.project(point_on_circle(center, radius, 0.0)),
            starts_segment: true,
        });
        for sample in 1..=samples {
            let theta = sample as f32 / samples as f32 * std::f32::consts::TAU;
            points.push(ProjectedPathPoint {
                point: self.project(point_on_circle(center, radius, theta)),
                starts_segment: false,
            });
        }
        self.stroke_edge_blurred_polyline(frame, &points, stroke);
    }

    fn fill_edge_blurred_circle(
        self,
        frame: &mut canvas::Frame,
        center: Point,
        radius: f32,
        samples: usize,
        color: Color,
    ) {
        let blur_factor = self.edge_blur_factor_for_circle(center, radius);
        if blur_factor <= f32::EPSILON {
            return;
        }

        let boundary = self.circle_path(center, radius, samples, ChromaticChannel::Main);
        self.stroke_soft_boundary(frame, &boundary, color, blur_factor, EDGE_BLUR_FILL_ALPHA);
    }

    fn fill_edge_blurred_polygon(self, frame: &mut canvas::Frame, points: &[Point], color: Color) {
        let blur_factor = self.edge_blur_factor_for_points(points);
        if blur_factor <= f32::EPSILON {
            return;
        }

        let boundary = self.polygon_path(points, ChromaticChannel::Main);
        self.stroke_soft_boundary(frame, &boundary, color, blur_factor, EDGE_BLUR_FILL_ALPHA);
    }

    fn stroke_edge_blurred_path_points<'a>(
        self,
        frame: &mut canvas::Frame,
        points: &[ProjectedPathPoint],
        stroke: canvas::Stroke<'a>,
    ) {
        let projected: Vec<ProjectedPathPoint> = points
            .iter()
            .map(|point| ProjectedPathPoint {
                point: self.project(point.point),
                starts_segment: point.starts_segment,
            })
            .collect();
        self.stroke_edge_blurred_polyline(frame, &projected, stroke);
    }

    fn stroke_chromatic_line<'a>(
        self,
        frame: &mut canvas::Frame,
        start: Point,
        end: Point,
        samples: usize,
        stroke: canvas::Stroke<'a>,
    ) {
        let red_path = self.line_path(start, end, samples, ChromaticChannel::Red);
        let cyan_path = self.line_path(start, end, samples, ChromaticChannel::Cyan);
        let source_color = stroke_color(&stroke);
        frame.stroke(
            &red_path,
            stroke.with_color(self.chromatic_color(
                source_color,
                ChromaticChannel::Red,
                CHROMATIC_STROKE_ALPHA,
            )),
        );
        frame.stroke(
            &cyan_path,
            stroke.with_color(self.chromatic_color(
                source_color,
                ChromaticChannel::Cyan,
                CHROMATIC_STROKE_ALPHA,
            )),
        );
    }

    fn line_path(
        self,
        start: Point,
        end: Point,
        samples: usize,
        channel: ChromaticChannel,
    ) -> canvas::Path {
        canvas::Path::new(|path| {
            path.move_to(self.visual_point(start, channel));
            for sample in 1..=samples {
                let t = sample as f32 / samples as f32;
                path.line_to(self.visual_point(lerp_point(start, end, t), channel));
            }
        })
    }

    fn circle_path(
        self,
        center: Point,
        radius: f32,
        samples: usize,
        channel: ChromaticChannel,
    ) -> canvas::Path {
        canvas::Path::new(|path| {
            let first = point_on_circle(center, radius, 0.0);
            path.move_to(self.visual_point(first, channel));
            for sample in 1..=samples {
                let theta = sample as f32 / samples as f32 * std::f32::consts::TAU;
                path.line_to(self.visual_point(point_on_circle(center, radius, theta), channel));
            }
            path.close();
        })
    }

    fn path_points(self, points: &[ProjectedPathPoint], channel: ChromaticChannel) -> canvas::Path {
        canvas::Path::new(|path| {
            for point in points {
                let projected = self.visual_point(point.point, channel);
                if point.starts_segment {
                    path.move_to(projected);
                } else {
                    path.line_to(projected);
                }
            }
        })
    }

    fn polygon_path(self, points: &[Point], channel: ChromaticChannel) -> canvas::Path {
        canvas::Path::new(|path| {
            let Some(first) = points.first() else {
                return;
            };
            path.move_to(self.visual_point(*first, channel));
            for point in &points[1..] {
                path.line_to(self.visual_point(*point, channel));
            }
            path.close();
        })
    }

    fn visual_point(self, point: Point, channel: ChromaticChannel) -> Point {
        let projected = self.project(point);
        match channel {
            ChromaticChannel::Main => projected,
            ChromaticChannel::Red => self.chromatic_point(projected, 1.0),
            ChromaticChannel::Cyan => self.chromatic_point(projected, -1.0),
        }
    }

    fn chromatic_point(self, point: Point, direction_sign: f32) -> Point {
        if !self.chromatic_enabled {
            return point;
        }

        let center = Point::new(self.width * 0.5, self.height * 0.5);
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= f32::EPSILON || !len.is_finite() {
            return point;
        }

        let radius = self.normalized_radius(point);
        let shift =
            direction_sign * self.chromatic_strength * MAX_CHROMATIC_SHIFT_PX * radius.powf(1.2);
        self.clamp_projected_point(Point::new(
            point.x + dx / len * shift,
            point.y + dy / len * shift,
        ))
    }

    fn chromatic_color(self, source: Color, channel: ChromaticChannel, alpha_scale: f32) -> Color {
        // Real lateral chromatic aberration separates the source's own color
        // channels, so each fringe carries only its channel and fades with the
        // energy the source has in it (a pure-green candle shows no red fringe).
        // Channel values are sqrt-boosted: the isolated channel reads dimmer
        // than the composite color, so partial channels need brightening to
        // stay visible without reintroducing fringes the source can't produce.
        let alpha = |channel_energy: f32| {
            (source.a * alpha_scale * self.chromatic_strength * channel_energy.max(0.0).sqrt())
                .clamp(0.0, CHROMATIC_ALPHA_CEILING)
        };
        let boost = |value: f32| value.max(0.0).sqrt();
        match channel {
            ChromaticChannel::Main => source,
            ChromaticChannel::Red => Color {
                r: boost(source.r),
                g: 0.0,
                b: 0.0,
                a: alpha(source.r),
            },
            ChromaticChannel::Cyan => Color {
                r: 0.0,
                g: boost(source.g),
                b: boost(source.b),
                a: alpha(source.g.max(source.b)),
            },
        }
    }

    fn edge_blur_color(self, source: Color, alpha_scale: f32, edge_factor: f32) -> Color {
        Color {
            a: (source.a * alpha_scale * self.edge_blur_strength * edge_factor)
                .clamp(0.0, EDGE_BLUR_ALPHA_CEILING),
            ..source
        }
    }

    fn edge_blur_factor(self, point: Point) -> f32 {
        if !self.edge_blur_enabled {
            return 0.0;
        }
        let t = ((self.normalized_radius(point).clamp(0.0, 1.0) - EDGE_BLUR_INNER_RADIUS)
            / EDGE_BLUR_RADIUS_RANGE)
            .clamp(0.0, 1.0);
        smoothstep(t)
    }

    fn edge_blur_factor_for_points(self, points: &[Point]) -> f32 {
        points
            .iter()
            .map(|point| self.edge_blur_factor(self.project(*point)))
            .fold(0.0_f32, f32::max)
    }

    fn edge_blur_factor_for_rect(self, origin: Point, size: Size) -> f32 {
        self.edge_blur_factor_for_points(&[
            origin,
            Point::new(origin.x + size.width, origin.y),
            Point::new(origin.x + size.width, origin.y + size.height),
            Point::new(origin.x, origin.y + size.height),
        ])
    }

    fn edge_blur_factor_for_circle(self, center: Point, radius: f32) -> f32 {
        self.edge_blur_factor_for_points(&[
            Point::new(center.x + radius, center.y),
            Point::new(center.x - radius, center.y),
            Point::new(center.x, center.y + radius),
            Point::new(center.x, center.y - radius),
        ])
    }

    fn coefficient(self) -> f32 {
        self.strength * MAX_BARREL_COEFFICIENT
    }

    fn without_edge_blur(self) -> Self {
        Self {
            edge_blur_enabled: false,
            edge_blur_strength: 0.0,
            ..self
        }
    }

    fn center(self) -> Point {
        Point::new(self.width * 0.5, self.height * 0.5)
    }

    fn normalized_radius(self, point: Point) -> f32 {
        let center = self.center();
        let half_w = self.width * 0.5;
        let half_h = self.height * 0.5;
        if half_w <= 0.0 || half_h <= 0.0 {
            return 0.0;
        }

        (((point.x - center.x) / half_w).powi(2) + ((point.y - center.y) / half_h).powi(2))
            .sqrt()
            .clamp(0.0, 1.35)
    }

    fn contains_point(self, point: Point) -> bool {
        point.x.is_finite()
            && point.y.is_finite()
            && point.x >= 0.0
            && point.x <= self.width
            && point.y >= 0.0
            && point.y <= self.height
    }

    fn contains_projection_y(self, y: f32) -> bool {
        y.is_finite() && y >= 0.0 && y <= self.height
    }

    fn to_normalized(self, point: Point) -> (f32, f32) {
        let half_w = self.width * 0.5;
        let half_h = self.height * 0.5;
        ((point.x - half_w) / half_w, (point.y - half_h) / half_h)
    }

    fn denormalize_point(self, x: f32, y: f32) -> Point {
        Point::new(
            self.width * 0.5 + x * self.width * 0.5,
            self.height * 0.5 + y * self.height * 0.5,
        )
    }

    fn clamp_projected_point(self, point: Point) -> Point {
        Point::new(
            point.x.clamp(0.0, self.width),
            point.y.clamp(0.0, self.height),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedPathPoint {
    pub(crate) point: Point,
    pub(crate) starts_segment: bool,
}

#[derive(Debug, Clone, Copy)]
struct EdgeBlurRect {
    origin: Point,
    size: Size,
    cols: usize,
    rows: usize,
}

#[derive(Debug, Clone, Copy)]
enum ChromaticChannel {
    Main,
    Red,
    Cyan,
}

#[derive(Debug, Clone, Copy)]
struct EdgeBlurLayer {
    radius_scale: f32,
    alpha_scale: f32,
}

fn path_from_runs(runs: &[Vec<Point>]) -> canvas::Path {
    canvas::Path::new(|path| {
        for run in runs {
            let mut points = run.iter();
            let Some(first) = points.next() else {
                continue;
            };
            path.move_to(*first);
            for point in points {
                path.line_to(*point);
            }
        }
    })
}

fn line_samples(start: Point, end: Point) -> usize {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    ((dx * dx + dy * dy).sqrt() / LINE_SAMPLE_PX)
        .ceil()
        .clamp(2.0, MAX_LINE_SAMPLES as f32) as usize
}

fn rect_subdivisions(length: f32) -> usize {
    (length / 22.0).ceil().clamp(1.0, 8.0) as usize
}

fn point_on_circle(center: Point, radius: f32, theta: f32) -> Point {
    Point::new(
        center.x + radius * theta.cos(),
        center.y + radius * theta.sin(),
    )
}

fn lerp_point(start: Point, end: Point, t: f32) -> Point {
    Point::new(
        start.x + (end.x - start.x) * t,
        start.y + (end.y - start.y) * t,
    )
}

fn valid_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn valid_rect(origin: Point, size: Size) -> bool {
    valid_point(origin)
        && size.width > 0.0
        && size.height > 0.0
        && size.width.is_finite()
        && size.height.is_finite()
}

fn edge_blur_bucket(edge_factor: f32) -> Option<usize> {
    if edge_factor <= f32::EPSILON || !edge_factor.is_finite() {
        return None;
    }

    Some(((edge_factor * EDGE_BLUR_BUCKETS as f32) as usize).min(EDGE_BLUR_BUCKETS - 1))
}

// Bucket midpoints rather than upper edges: a factor barely past the inner
// radius should get a whisper of blur, not jump a full bucket's worth.
fn edge_blur_bucket_factor(bucket: usize) -> f32 {
    (bucket as f32 + 0.5) / EDGE_BLUR_BUCKETS as f32
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn stroke_color(stroke: &canvas::Stroke<'_>) -> Color {
    match stroke.style {
        canvas::Style::Solid(color) => color,
        canvas::Style::Gradient(_) => Color::WHITE,
    }
}

#[cfg(test)]
mod tests {
    use super::{CHROMATIC_STROKE_ALPHA, ChartFisheye, ChromaticChannel, ProjectedPathPoint};
    use iced::{Color, Point, Size};

    #[test]
    fn disabled_projection_is_identity() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0);
        let point = Point::new(120.0, 80.0);

        assert_eq!(lens.project(point), point);
        assert_eq!(lens.unproject(point), point);
    }

    #[test]
    fn chromatic_aberration_does_not_move_main_geometry() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_chromatic(true, 0.8);
        let point = Point::new(120.0, 80.0);

        assert!(lens.is_enabled());
        assert_eq!(lens.project(point), point);
        assert_eq!(lens.unproject(point), point);
    }

    #[test]
    fn chromatic_aberration_increases_away_from_center() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_chromatic(true, 1.0);
        let center = Point::new(400.0, 200.0);
        let near_center = Point::new(500.0, 200.0);
        let near_edge = Point::new(700.0, 200.0);

        let center_shift = lens.chromatic_point(center, 1.0).x - center.x;
        let near_center_shift = lens.chromatic_point(near_center, 1.0).x - near_center.x;
        let near_edge_shift = lens.chromatic_point(near_edge, 1.0).x - near_edge.x;

        assert!(center_shift.abs() <= f32::EPSILON);
        assert!(near_center_shift > center_shift);
        assert!(near_edge_shift > near_center_shift);
    }

    #[test]
    fn chromatic_fringes_carry_only_source_channels() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_chromatic(true, 1.0);
        let source = Color::from_rgb(0.9, 0.6, 0.3);

        let red = lens.chromatic_color(source, ChromaticChannel::Red, 1.0);
        assert_eq!((red.g, red.b), (0.0, 0.0));
        assert!(red.r >= source.r && red.a > 0.0);

        let cyan = lens.chromatic_color(source, ChromaticChannel::Cyan, 1.0);
        assert_eq!(cyan.r, 0.0);
        assert!(cyan.g >= source.g && cyan.b >= source.b && cyan.a > 0.0);
    }

    #[test]
    fn max_strength_chromatic_fringe_stays_prominent() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_chromatic(true, 1.0);

        let white_stroke =
            lens.chromatic_color(Color::WHITE, ChromaticChannel::Red, CHROMATIC_STROKE_ALPHA);
        assert!(white_stroke.a >= 0.4);

        let near_edge = Point::new(780.0, 200.0);
        let shift = (lens.chromatic_point(near_edge, 1.0).x - near_edge.x).abs();
        assert!(shift >= 5.0);
    }

    #[test]
    fn chromatic_fringe_vanishes_when_source_lacks_the_channel() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_chromatic(true, 1.0);

        let green = Color::from_rgb(0.0, 0.85, 0.0);
        let red_fringe = lens.chromatic_color(green, ChromaticChannel::Red, 1.0);
        assert!(red_fringe.a <= f32::EPSILON);

        let red = Color::from_rgb(0.85, 0.0, 0.0);
        let cyan_fringe = lens.chromatic_color(red, ChromaticChannel::Cyan, 1.0);
        assert!(cyan_fringe.a <= f32::EPSILON);
    }

    #[test]
    fn edge_blur_increases_toward_outer_edges() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_edge_blur(true, 1.0);
        let center = Point::new(400.0, 200.0);
        let near_center = Point::new(500.0, 200.0);
        let near_edge = Point::new(700.0, 200.0);

        assert!(lens.is_enabled());
        assert!(lens.edge_blur_factor(center) <= f32::EPSILON);
        assert!(lens.edge_blur_factor(near_center) > lens.edge_blur_factor(center));
        assert!(lens.edge_blur_factor(near_edge) > lens.edge_blur_factor(near_center));
    }

    #[test]
    fn edge_blur_runs_skip_the_sharp_chart_center() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_edge_blur(true, 1.0);
        let points: Vec<ProjectedPathPoint> = (0..=32)
            .map(|i| ProjectedPathPoint {
                point: Point::new(i as f32 * 25.0, 200.0),
                starts_segment: i == 0,
            })
            .collect();

        let runs = lens.edge_blur_polyline_runs(&points);
        let run_points: Vec<Point> = runs.iter().flatten().flatten().copied().collect();

        assert!(!run_points.is_empty());
        for point in &run_points {
            assert!(
                (point.x - 400.0).abs() >= 60.0,
                "blur geometry reached the sharp center: x={}",
                point.x
            );
        }
    }

    #[test]
    fn edge_blur_buckets_start_gently_and_grade_smoothly() {
        let low_bucket = super::edge_blur_bucket(0.01).expect("small factors still bucket");
        assert!(super::edge_blur_bucket_factor(low_bucket) < 0.15);

        let high_bucket = super::edge_blur_bucket(1.0).expect("full factor buckets");
        assert!(super::edge_blur_bucket_factor(high_bucket) <= 1.0);
        for bucket in 0..high_bucket {
            assert!(
                super::edge_blur_bucket_factor(bucket) < super::edge_blur_bucket_factor(bucket + 1)
            );
        }
    }

    #[test]
    fn edge_blur_does_not_move_main_geometry() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_edge_blur(true, 0.8);
        let point = Point::new(120.0, 80.0);

        assert_eq!(lens.project(point), point);
        assert_eq!(lens.unproject(point), point);
    }

    #[test]
    fn without_edge_blur_preserves_projection_and_chromatic_settings() {
        let lens = ChartFisheye::new(true, 0.7, 800.0, 400.0)
            .with_chromatic(true, 0.6)
            .with_edge_blur(true, 0.8);
        let unblurred = lens.without_edge_blur();

        assert!(unblurred.enabled);
        assert_eq!(unblurred.strength, lens.strength);
        assert!(unblurred.chromatic_enabled);
        assert_eq!(unblurred.chromatic_strength, lens.chromatic_strength);
        assert!(!unblurred.edge_blur_enabled);
        assert_eq!(unblurred.edge_blur_strength, 0.0);
    }

    #[test]
    fn edge_blur_rect_buckets_skip_center_and_invalid_rects() {
        let lens = ChartFisheye::new(false, 1.0, 800.0, 400.0).with_edge_blur(true, 1.0);
        let buckets = lens.edge_blur_rect_buckets(&[
            (Point::new(399.0, 199.0), Size::new(2.0, 2.0)),
            (Point::new(690.0, 190.0), Size::new(8.0, 8.0)),
            (Point::new(f32::NAN, 0.0), Size::new(8.0, 8.0)),
        ]);

        let bucketed_rects: usize = buckets.iter().map(Vec::len).sum();
        assert_eq!(bucketed_rects, 1);
        assert!(buckets.iter().any(|bucket| bucket.len() == 1));
    }

    #[test]
    fn projection_is_clamped_to_chart_bounds() {
        let lens = ChartFisheye::new(true, 1.0, 800.0, 400.0);

        for point in [
            Point::new(0.0, 0.0),
            Point::new(800.0, 0.0),
            Point::new(800.0, 400.0),
            Point::new(0.0, 400.0),
        ] {
            let projected = lens.project(point);
            assert!(projected.x >= 0.0 && projected.x <= 800.0);
            assert!(projected.y >= 0.0 && projected.y <= 400.0);
        }
    }

    #[test]
    fn projection_round_trips_inside_chart_area() {
        let lens = ChartFisheye::new(true, 0.7, 800.0, 400.0);

        for point in [
            Point::new(400.0, 200.0),
            Point::new(120.0, 80.0),
            Point::new(720.0, 320.0),
            Point::new(400.0, 40.0),
        ] {
            let round_trip = lens.unproject(lens.project(point));
            assert!((round_trip.x - point.x).abs() < 0.02);
            assert!((round_trip.y - point.y).abs() < 0.02);
        }
    }
}
