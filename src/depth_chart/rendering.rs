use super::geometry::{
    DepthChartLayout, axis_size_label, depth_chart_layout, hover_target, marker_xs, side_points,
};
use super::{DepthChart, DepthChartState};
use crate::helpers::{format_decimal_with_commas, format_size};
use iced::widget::canvas::{self, Frame, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Depth Chart Rendering
// ---------------------------------------------------------------------------

const FILL_ALPHA: f32 = 0.15;
const SIZE_AXIS_FRACTIONS: [f64; 4] = [0.25, 0.5, 0.75, 1.0];
const PRICE_AXIS_FRACTIONS: [f32; 4] = [0.1, 0.3, 0.7, 0.9];
const PRICE_LABEL_BASELINE_INSET: f32 = 2.0;
/// Vertical band at the bottom occupied by the price labels. The depth fills
/// run to the bottom edge underneath it, but dashed guides and order markers
/// stop above it so the labels stay clean.
const PRICE_LABEL_BAND: f32 = 14.0;
const ORDER_MARKER_HEIGHT: f32 = 5.0;
const ORDER_MARKER_HALF_WIDTH: f32 = 3.5;

impl DepthChart {
    pub(super) fn draw_chart(
        &self,
        state: &DepthChartState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let Some(layout) = self.layout(bounds) else {
            return vec![frame.into_geometry()];
        };

        draw_side(
            &mut frame,
            &side_points(&self.bids, &layout, true),
            theme.palette().success,
        );
        draw_side(
            &mut frame,
            &side_points(&self.asks, &layout, false),
            theme.palette().danger,
        );

        let muted = theme.extended_palette().background.weak.text;
        self.draw_mid_marker(&mut frame, &layout, theme, muted);
        draw_size_axis(&mut frame, &layout, muted);
        self.draw_price_axis(&mut frame, &layout, muted);
        draw_order_markers(
            &mut frame,
            &marker_xs(&self.user_bid_prices, &layout),
            &layout,
            theme.palette().success,
        );
        draw_order_markers(
            &mut frame,
            &marker_xs(&self.user_ask_prices, &layout),
            &layout,
            theme.palette().danger,
        );

        if let Some(hover_pos) = state.hover_pos {
            self.draw_hover(&mut frame, &layout, theme, muted, hover_pos);
        }

        vec![frame.into_geometry()]
    }

    /// Paint scale for the current data and bounds; `None` when there is
    /// nothing to draw (no mid price or no levels).
    pub(super) fn layout(&self, bounds: Rectangle) -> Option<DepthChartLayout> {
        let mid = self.mid?;
        depth_chart_layout(
            &self.bids,
            &self.asks,
            mid,
            self.tick,
            bounds.width,
            bounds.height,
        )
    }

    fn draw_mid_marker(
        &self,
        frame: &mut Frame,
        layout: &DepthChartLayout,
        theme: &Theme,
        muted: Color,
    ) {
        let x = layout.x_for_price(layout.mid);
        let marker = canvas::Path::line(
            Point::new(x, 0.0),
            Point::new(x, layout.height - PRICE_LABEL_BAND),
        );
        let mut stroke = Stroke::default()
            .with_color(Color { a: 0.5, ..muted })
            .with_width(1.0);
        stroke.line_dash = canvas::stroke::LineDash {
            segments: &[4.0, 4.0],
            offset: 0,
        };
        frame.stroke(&marker, stroke);

        frame.fill_text(canvas::Text {
            content: format_decimal_with_commas(layout.mid, self.decimals),
            position: Point::new(x, layout.height - PRICE_LABEL_BASELINE_INSET),
            color: theme.palette().text,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Bottom,
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }

    fn draw_price_axis(&self, frame: &mut Frame, layout: &DepthChartLayout, muted: Color) {
        for fraction in PRICE_AXIS_FRACTIONS {
            let x = layout.width * fraction;
            frame.fill_text(canvas::Text {
                content: format_decimal_with_commas(layout.price_at_x(x), self.decimals),
                position: Point::new(x, layout.height - PRICE_LABEL_BASELINE_INSET),
                color: muted,
                size: iced::Pixels(10.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Bottom,
                font: crate::app_fonts::monospace_font(),
                ..Default::default()
            });
        }
    }

    fn draw_hover(
        &self,
        frame: &mut Frame,
        layout: &DepthChartLayout,
        theme: &Theme,
        muted: Color,
        hover_pos: Point,
    ) {
        let guide = canvas::Path::line(
            Point::new(hover_pos.x, 0.0),
            Point::new(hover_pos.x, layout.height - PRICE_LABEL_BAND),
        );
        let mut stroke = Stroke::default()
            .with_color(Color { a: 0.5, ..muted })
            .with_width(1.0);
        stroke.line_dash = canvas::stroke::LineDash {
            segments: &[4.0, 4.0],
            offset: 0,
        };
        frame.stroke(&guide, stroke);

        let Some(target) = hover_target(&self.bids, &self.asks, layout, hover_pos.x) else {
            return;
        };
        let color = if target.is_bid {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        let y = layout.y_for_cum(target.cum);
        frame.fill(
            &canvas::Path::circle(Point::new(hover_pos.x, y), 3.0),
            color,
        );

        frame.fill_text(canvas::Text {
            content: format!(
                "{} | {}",
                format_decimal_with_commas(target.price, self.decimals),
                format_size(target.cum)
            ),
            position: Point::new(
                hover_pos.x.max(50.0).min(layout.width - 50.0),
                (y - 8.0).max(12.0),
            ),
            color: theme.palette().text,
            size: iced::Pixels(11.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Bottom,
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
}

/// Fill and stroke one side's step curve. `points` start at the baseline by
/// construction, so closing the area path back to the first point runs along
/// the bottom edge.
fn draw_side(frame: &mut Frame, points: &[Point], color: Color) {
    if points.len() < 2 {
        return;
    }

    let mut line = canvas::path::Builder::new();
    let mut area = canvas::path::Builder::new();
    line.move_to(points[0]);
    area.move_to(points[0]);
    for point in &points[1..] {
        line.line_to(*point);
        area.line_to(*point);
    }
    let last = points[points.len() - 1];
    area.line_to(Point::new(last.x, points[0].y));
    area.close();

    frame.fill(
        &area.build(),
        Color {
            a: FILL_ALPHA,
            ..color
        },
    );
    frame.stroke(
        &line.build(),
        Stroke::default().with_width(1.5).with_color(color),
    );
}

/// Small upward triangles just above the price-label band marking the tick
/// buckets that hold the user's resting orders.
fn draw_order_markers(frame: &mut Frame, xs: &[f32], layout: &DepthChartLayout, color: Color) {
    let base_y = layout.height - PRICE_LABEL_BAND;
    for &x in xs {
        let mut marker = canvas::path::Builder::new();
        marker.move_to(Point::new(x, base_y - ORDER_MARKER_HEIGHT));
        marker.line_to(Point::new(x - ORDER_MARKER_HALF_WIDTH, base_y));
        marker.line_to(Point::new(x + ORDER_MARKER_HALF_WIDTH, base_y));
        marker.close();
        frame.fill(&marker.build(), color);
    }
}

fn draw_size_axis(frame: &mut Frame, layout: &DepthChartLayout, muted: Color) {
    for fraction in SIZE_AXIS_FRACTIONS {
        let cum = layout.max_cum * fraction;
        frame.fill_text(canvas::Text {
            content: axis_size_label(cum),
            position: Point::new(layout.width - 4.0, layout.y_for_cum(cum)),
            color: muted,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
}
