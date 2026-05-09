use super::{SpreadChart, SpreadChartState};
use iced::widget::canvas::{self, Frame, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme};

mod points;

use points::{SpreadChartScale, closest_spread_point, rendered_spread_points};

impl SpreadChart<'_> {
    pub(super) fn draw_chart(
        &self,
        state: &SpreadChartState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.data.is_empty() {
            return vec![frame.into_geometry()];
        }

        let w = bounds.width;
        let h = bounds.height;
        let scale = SpreadChartScale::new(self.data, std::time::Instant::now(), w, h);

        let base_color = theme.extended_palette().background.weak.text;

        let mut path_builder = canvas::path::Builder::new();
        let mut first = true;
        let mut first_pt = Point::ORIGIN;
        let mut last_pt = Point::ORIGIN;
        let mut has_points = false;

        let rendered_points = rendered_spread_points(self.data, &scale);

        for (pt, _) in &rendered_points {
            if first {
                path_builder.move_to(*pt);
                first_pt = *pt;
                first = false;
            } else {
                path_builder.line_to(*pt);
            }
            last_pt = *pt;
            has_points = true;
        }

        if !has_points {
            return vec![frame.into_geometry()];
        }

        let line_path = path_builder.build();

        if !first {
            let mut fill_builder = canvas::path::Builder::new();
            fill_builder.move_to(Point::new(first_pt.x, h));
            fill_builder.line_to(first_pt);

            for (pt, _) in rendered_points.iter().skip(1) {
                fill_builder.line_to(*pt);
            }

            fill_builder.line_to(Point::new(last_pt.x, h));
            fill_builder.close();

            let fill_path = fill_builder.build();

            let fill_color = Color {
                a: 0.15,
                ..base_color
            };
            frame.fill(&fill_path, fill_color);
        }

        frame.stroke(
            &line_path,
            Stroke::default().with_width(1.5).with_color(base_color),
        );

        if let Some(hover_pos) = state.hover_pos {
            draw_hover_state(HoverRenderContext {
                chart: self,
                frame: &mut frame,
                theme,
                h,
                w,
                base_color,
                rendered_points: &rendered_points,
                hover_pos,
            });
        } else if let Some((_, current_spread)) = self.data.front() {
            let text = canvas::Text {
                content: format!(
                    "Spread: {:.prec$}",
                    current_spread,
                    prec = self.spread_decimals
                ),
                position: Point::new(w - 4.0, 4.0),
                color: theme.extended_palette().background.weak.text,
                size: iced::Pixels(11.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Top,
                ..Default::default()
            };
            frame.fill_text(text);
        }

        vec![frame.into_geometry()]
    }
}

struct HoverRenderContext<'a, 'data> {
    chart: &'a SpreadChart<'data>,
    frame: &'a mut Frame,
    theme: &'a Theme,
    h: f32,
    w: f32,
    base_color: Color,
    rendered_points: &'a [(Point, f64)],
    hover_pos: Point,
}

fn draw_hover_state(ctx: HoverRenderContext<'_, '_>) {
    if let Some((pt, spread)) = closest_spread_point(ctx.rendered_points, ctx.hover_pos) {
        let v_line = canvas::Path::line(Point::new(pt.x, 0.0), Point::new(pt.x, ctx.h));
        let mut stroke = Stroke::default()
            .with_color(Color {
                a: 0.5,
                ..ctx.base_color
            })
            .with_width(1.0);
        stroke.line_dash = canvas::stroke::LineDash {
            segments: &[4.0, 4.0],
            offset: 0,
        };
        ctx.frame.stroke(&v_line, stroke);

        let dot = canvas::Path::circle(pt, 3.0);
        ctx.frame.fill(&dot, ctx.base_color);

        let text = canvas::Text {
            content: format!("{:.prec$}", spread, prec = ctx.chart.spread_decimals),
            position: Point::new(pt.x.max(5.0).min(ctx.w - 30.0), pt.y.max(15.0) - 10.0),
            color: ctx.theme.palette().text,
            size: iced::Pixels(11.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Bottom,
            ..Default::default()
        };
        ctx.frame.fill_text(text);
    }
}
