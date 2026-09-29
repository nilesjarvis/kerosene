use crate::denomination::DisplayDenominationContext;
use crate::liquidations_distribution_state::{
    LIQUIDATION_DISTRIBUTION_ZOOM_STEP, LiquidationDistributionData, LiquidationDistributionPoint,
    LiquidationDistributionZoomAnchor,
};
use crate::message::Message;
use iced::widget::canvas::{self, Frame, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme, color, mouse};

mod labels;
use labels::{HoverStateRenderContext, draw_axes, draw_current_mark, draw_hover_state};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Liquidations Distribution Canvas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct LiquidationsDistributionChart<'a> {
    pub(super) data: &'a LiquidationDistributionData,
    pub(super) denomination: DisplayDenominationContext,
    pub(super) zoom: f64,
    pub(super) zoom_center_price: Option<f64>,
}

impl canvas::Program<Message> for LiquidationsDistributionChart<'_> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event else {
            return None;
        };
        let dy = match delta {
            mouse::ScrollDelta::Lines { y, .. } => *y,
            mouse::ScrollDelta::Pixels { y, .. } => *y / 28.0,
        };
        if dy == 0.0 {
            return None;
        }
        let pos = cursor.position_in(bounds)?;
        let margins = ChartMargins::for_width(bounds.width);
        let domain = self.visible_price_range();
        let plot = PlotArea::new(bounds, margins, domain);
        let anchor = if plot.contains(pos) {
            Some(LiquidationDistributionZoomAnchor {
                price: plot.x_to_price(pos.x),
                fraction: ((pos.x - plot.left) / plot.width.max(1.0)) as f64,
            })
        } else {
            None
        };
        let factor = if dy > 0.0 {
            LIQUIDATION_DISTRIBUTION_ZOOM_STEP
        } else {
            1.0 / LIQUIDATION_DISTRIBUTION_ZOOM_STEP
        };

        Some(
            canvas::Action::publish(Message::LiquidationsDistributionZoomed { factor, anchor })
                .and_capture(),
        )
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        draw_distribution_chart(
            self.data,
            &self.denomination,
            renderer,
            theme,
            bounds,
            cursor,
            self.visible_price_range(),
        )
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.position_in(bounds).is_some() {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl LiquidationsDistributionChart<'_> {
    fn visible_price_range(&self) -> (f64, f64) {
        crate::liquidations_distribution_state::liquidation_distribution_visible_price_range(
            self.data,
            self.zoom,
            self.zoom_center_price,
        )
    }
}

fn draw_distribution_chart(
    data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
    price_domain: (f64, f64),
) -> Vec<canvas::Geometry> {
    let mut frame = Frame::new(renderer, bounds.size());
    if data.points.is_empty() || !data.has_values() || bounds.width < 180.0 || bounds.height < 120.0
    {
        draw_empty_chart(&mut frame, theme, bounds);
        return vec![frame.into_geometry()];
    }

    let margins = ChartMargins::for_width(bounds.width);
    let plot = PlotArea::new(bounds, margins, price_domain);
    if plot.width <= 0.0 || plot.height <= 0.0 {
        return vec![frame.into_geometry()];
    }

    let max_bucket_usd = visible_max_bucket_usd(data, &plot)
        .unwrap_or(data.max_bucket_usd)
        .max(1.0);
    let max_cumulative_usd = visible_max_cumulative_usd(data, &plot)
        .unwrap_or(data.max_cumulative_usd)
        .max(1.0);

    draw_grid(&mut frame, theme, &plot);
    frame.with_clip(plot.rectangle(), |frame| {
        draw_bars(frame, data, &plot, theme, max_bucket_usd);
        draw_cumulative_area(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            true,
            color!(0xff7777),
        );
        draw_cumulative_area(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            false,
            color!(0x66d9a8),
        );
        draw_cumulative_line(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            true,
            color!(0xff7777),
        );
        draw_cumulative_line(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            false,
            color!(0x66d9a8),
        );
    });
    draw_axes(
        &mut frame,
        data,
        denomination,
        theme,
        &plot,
        max_bucket_usd,
        max_cumulative_usd,
    );
    draw_current_mark(&mut frame, data, denomination, theme, &plot);
    draw_hover_state(HoverStateRenderContext {
        frame: &mut frame,
        data,
        denomination,
        theme,
        bounds,
        plot: &plot,
        cursor,
        max_cumulative_usd,
    });

    vec![frame.into_geometry()]
}

#[derive(Debug, Clone, Copy)]
struct ChartMargins {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl ChartMargins {
    fn for_width(width: f32) -> Self {
        if width >= 520.0 {
            Self {
                left: 58.0,
                right: 68.0,
                top: 14.0,
                bottom: 32.0,
            }
        } else {
            Self {
                left: 44.0,
                right: 48.0,
                top: 10.0,
                bottom: 28.0,
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PlotArea {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    width: f32,
    height: f32,
    price_min: f64,
    price_max: f64,
}

impl PlotArea {
    fn new(bounds: Rectangle, margins: ChartMargins, price_domain: (f64, f64)) -> Self {
        let left = margins.left;
        let right = (bounds.width - margins.right).max(left);
        let top = margins.top;
        let bottom = (bounds.height - margins.bottom).max(top);
        let mut price_min = price_domain.0.min(price_domain.1);
        let mut price_max = price_domain.0.max(price_domain.1);
        if !price_min.is_finite() || !price_max.is_finite() || price_max <= price_min {
            price_min = 0.0;
            price_max = price_min + 1.0;
        }
        Self {
            left,
            right,
            top,
            bottom,
            width: right - left,
            height: bottom - top,
            price_min,
            price_max,
        }
    }

    fn price_to_x(self, price: f64) -> f32 {
        let range = self.price_max - self.price_min;
        if range <= 0.0 {
            return self.left;
        }
        let ratio = ((price - self.price_min) / range) as f32;
        self.left + ratio * self.width
    }

    fn x_to_price(self, x: f32) -> f64 {
        let range = self.price_max - self.price_min;
        if range <= 0.0 || self.width <= 0.0 {
            return self.price_min;
        }
        let fraction = ((x - self.left) / self.width).clamp(0.0, 1.0) as f64;
        self.price_min + range * fraction
    }

    fn contains(self, point: Point) -> bool {
        point.x >= self.left
            && point.x <= self.right
            && point.y >= self.top
            && point.y <= self.bottom
    }

    fn price_is_visible(self, price: f64) -> bool {
        price >= self.price_min && price <= self.price_max
    }

    fn rectangle(self) -> Rectangle {
        Rectangle {
            x: self.left,
            y: self.top,
            width: self.width,
            height: self.height,
        }
    }

    fn value_to_y(self, value: f64, max_value: f64) -> f32 {
        if max_value <= 0.0 {
            return self.bottom;
        }
        let ratio = (value / max_value).clamp(0.0, 1.0) as f32;
        self.bottom - ratio * self.height
    }
}

fn draw_empty_chart(frame: &mut Frame, theme: &Theme, bounds: Rectangle) {
    frame.fill_text(canvas::Text {
        content: "No liquidation levels in range".to_string(),
        position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
        color: theme.extended_palette().background.weak.text,
        size: iced::Pixels(12.0),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

fn draw_grid(frame: &mut Frame, theme: &Theme, plot: &PlotArea) {
    let grid_color = Color {
        a: 0.09,
        ..theme.palette().text
    };
    for fraction in [0.25_f32, 0.5, 0.75] {
        let y = plot.top + plot.height * fraction;
        let path = canvas::Path::line(Point::new(plot.left, y), Point::new(plot.right, y));
        frame.stroke(
            &path,
            Stroke::default().with_color(grid_color).with_width(1.0),
        );
    }
    for fraction in [0.2_f32, 0.4, 0.6, 0.8] {
        let x = plot.left + plot.width * fraction;
        let path = canvas::Path::line(Point::new(x, plot.top), Point::new(x, plot.bottom));
        frame.stroke(
            &path,
            Stroke::default().with_color(grid_color).with_width(1.0),
        );
    }
}

fn draw_bars(
    frame: &mut Frame,
    data: &LiquidationDistributionData,
    plot: &PlotArea,
    theme: &Theme,
    max_bucket: f64,
) {
    let bucket_width = visible_bucket_width(&data.points, plot);
    let long_color = Color {
        a: 0.62,
        ..theme.palette().danger
    };
    let short_color = Color {
        a: 0.62,
        ..theme.palette().success
    };

    for point in &data.points {
        let x = plot.price_to_x(point.price);
        let bar_w = (bucket_width * 0.72).max(1.0);
        if x + bar_w < plot.left || x - bar_w > plot.right {
            continue;
        }

        if point.long_usd > 0.0 {
            let y = plot.value_to_y(point.long_usd, max_bucket);
            frame.fill_rectangle(
                Point::new(x - bar_w / 2.0, y),
                iced::Size::new(bar_w, plot.bottom - y),
                long_color,
            );
        }
        if point.short_usd > 0.0 {
            let y = plot.value_to_y(point.short_usd, max_bucket);
            frame.fill_rectangle(
                Point::new(x - bar_w / 2.0, y),
                iced::Size::new(bar_w, plot.bottom - y),
                short_color,
            );
        }
    }
}

fn draw_cumulative_area(
    frame: &mut Frame,
    points: &[LiquidationDistributionPoint],
    plot: &PlotArea,
    max_cumulative: f64,
    longs: bool,
    color: Color,
) {
    let visible: Vec<_> = points
        .iter()
        .filter(|point| {
            if !plot.price_is_visible(point.price) {
                return false;
            }
            if longs {
                point.cumulative_long_usd > 0.0
            } else {
                point.cumulative_short_usd > 0.0
            }
        })
        .collect();
    if visible.len() < 2 {
        return;
    }

    let mut area_color = color;
    area_color.a = 0.12;

    let path = canvas::Path::new(|builder| {
        if let Some(first) = visible.first() {
            builder.move_to(Point::new(point_x(first, plot), plot.bottom));
        }
        for point in &visible {
            let value = if longs {
                point.cumulative_long_usd
            } else {
                point.cumulative_short_usd
            };
            builder.line_to(Point::new(
                point_x(point, plot),
                plot.value_to_y(value, max_cumulative),
            ));
        }
        if let Some(last) = visible.last() {
            builder.line_to(Point::new(point_x(last, plot), plot.bottom));
        }
        builder.close();
    });
    frame.fill(&path, area_color);
}

fn draw_cumulative_line(
    frame: &mut Frame,
    points: &[LiquidationDistributionPoint],
    plot: &PlotArea,
    max_cumulative: f64,
    longs: bool,
    color: Color,
) {
    let mut builder = canvas::path::Builder::new();
    let mut has_point = false;
    for point in points {
        if !plot.price_is_visible(point.price) {
            continue;
        }
        let value = if longs {
            point.cumulative_long_usd
        } else {
            point.cumulative_short_usd
        };
        if value <= 0.0 {
            continue;
        }
        let p = Point::new(point_x(point, plot), plot.value_to_y(value, max_cumulative));
        if has_point {
            builder.line_to(p);
        } else {
            builder.move_to(p);
            has_point = true;
        }
    }
    if !has_point {
        return;
    }

    frame.stroke(
        &builder.build(),
        Stroke::default().with_width(1.8).with_color(color),
    );
}

fn point_x(point: &LiquidationDistributionPoint, plot: &PlotArea) -> f32 {
    plot.price_to_x(point.price)
}

fn visible_bucket_width(points: &[LiquidationDistributionPoint], plot: &PlotArea) -> f32 {
    let mut previous_x: Option<f32> = None;
    let mut min_step = f32::MAX;
    for point in points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
    {
        let x = plot.price_to_x(point.price);
        if let Some(previous_x) = previous_x {
            min_step = min_step.min((x - previous_x).abs());
        }
        previous_x = Some(x);
    }

    if min_step.is_finite() {
        min_step.max(1.0)
    } else {
        (plot.width / points.len().max(1) as f32).max(1.0)
    }
}

fn visible_max_bucket_usd(data: &LiquidationDistributionData, plot: &PlotArea) -> Option<f64> {
    data.points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
        .map(|point| point.long_usd.max(point.short_usd))
        .reduce(f64::max)
}

fn visible_max_cumulative_usd(data: &LiquidationDistributionData, plot: &PlotArea) -> Option<f64> {
    data.points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
        .map(|point| point.cumulative_long_usd.max(point.cumulative_short_usd))
        .reduce(f64::max)
}

fn nearest_distribution_point<'a>(
    data: &'a LiquidationDistributionData,
    plot: &PlotArea,
    cursor_x: f32,
) -> Option<&'a LiquidationDistributionPoint> {
    data.points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
        .min_by(|left, right| {
            let left_distance = (point_x(left, plot) - cursor_x).abs();
            let right_distance = (point_x(right, plot) - cursor_x).abs();
            left_distance.total_cmp(&right_distance)
        })
}
