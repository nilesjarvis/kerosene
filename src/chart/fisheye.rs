use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Chart Fisheye Projection
// ---------------------------------------------------------------------------

const MAX_BARREL_COEFFICIENT: f32 = 0.18;
const MAX_CHROMATIC_SHIFT_PX: f32 = 3.2;
const CHROMATIC_STROKE_ALPHA: f32 = 0.28;
const CHROMATIC_FILL_ALPHA: f32 = 0.14;
const LINE_SAMPLE_PX: f32 = 18.0;
const MAX_LINE_SAMPLES: usize = 96;
const NEWTON_STEPS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ChartFisheye {
    enabled: bool,
    strength: f32,
    chromatic_enabled: bool,
    chromatic_strength: f32,
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

    pub(crate) fn disabled() -> Self {
        Self {
            enabled: false,
            strength: 0.0,
            chromatic_enabled: false,
            chromatic_strength: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub(crate) fn is_enabled(self) -> bool {
        self.enabled || self.chromatic_enabled
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
        if self.chromatic_enabled {
            self.stroke_chromatic_line(frame, start, end, samples, stroke);
        }
        let path = self.line_path(start, end, samples, ChromaticChannel::Main);
        frame.stroke(&path, stroke);
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

        let radius = ((dx / (self.width * 0.5)).powi(2) + (dy / (self.height * 0.5)).powi(2))
            .sqrt()
            .clamp(0.0, 1.35);
        let shift =
            direction_sign * self.chromatic_strength * MAX_CHROMATIC_SHIFT_PX * radius.powf(1.35);
        self.clamp_projected_point(Point::new(
            point.x + dx / len * shift,
            point.y + dy / len * shift,
        ))
    }

    fn chromatic_color(self, source: Color, channel: ChromaticChannel, alpha_scale: f32) -> Color {
        let alpha = (source.a * alpha_scale * self.chromatic_strength).clamp(0.0, 0.38);
        match channel {
            ChromaticChannel::Main => source,
            ChromaticChannel::Red => Color {
                r: 1.0,
                g: 0.08,
                b: 0.03,
                a: alpha,
            },
            ChromaticChannel::Cyan => Color {
                r: 0.05,
                g: 0.68,
                b: 1.0,
                a: alpha,
            },
        }
    }

    fn coefficient(self) -> f32 {
        self.strength * MAX_BARREL_COEFFICIENT
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
enum ChromaticChannel {
    Main,
    Red,
    Cyan,
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

fn stroke_color(stroke: &canvas::Stroke<'_>) -> Color {
    match stroke.style {
        canvas::Style::Solid(color) => color,
        canvas::Style::Gradient(_) => Color::WHITE,
    }
}

#[cfg(test)]
mod tests {
    use super::ChartFisheye;
    use iced::Point;

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
