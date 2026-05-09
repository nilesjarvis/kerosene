use iced::widget::canvas;
use iced::{Color, Point, Size, Theme, alignment};

mod heatmap;
mod liquidations;

// ---------------------------------------------------------------------------
// Chart Tooltip Rendering
// ---------------------------------------------------------------------------

struct TooltipLine {
    content: String,
    color: Color,
}

pub(super) struct TooltipSurface<'a> {
    frame: &'a mut canvas::Frame,
    theme: &'a Theme,
    pos: Point,
    chart_w: f32,
    price_h: f32,
}

impl<'a> TooltipSurface<'a> {
    pub(super) fn new(
        frame: &'a mut canvas::Frame,
        theme: &'a Theme,
        pos: Point,
        chart_w: f32,
        price_h: f32,
    ) -> Self {
        Self {
            frame,
            theme,
            pos,
            chart_w,
            price_h,
        }
    }

    fn draw_card(&mut self, origin: Point, size: Size, lines: &[TooltipLine]) {
        let pad: f32 = 6.0;
        let line_h: f32 = 14.0;

        self.frame.fill_rectangle(
            origin,
            size,
            Color {
                a: 0.92,
                ..self.theme.extended_palette().background.strong.color
            },
        );

        let border = canvas::Path::rectangle(origin, size);
        self.frame.stroke(
            &border,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.15,
                    ..self.theme.palette().text
                })
                .with_width(1.0),
        );

        for (index, line) in lines.iter().enumerate() {
            self.frame.fill_text(canvas::Text {
                content: line.content.clone(),
                position: Point::new(
                    origin.x + pad,
                    origin.y + pad + index as f32 * line_h + line_h * 0.5,
                ),
                color: line.color,
                size: iced::Pixels(10.0),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Center,
                font: iced::Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }
    }
}
