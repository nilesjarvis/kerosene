use super::display_text::PnlCardRenderText;
use super::style::{PnlCardPalette, pnl_card_palette};

use iced::alignment;
use iced::font::Weight;
use iced::widget::canvas;
use iced::{Color, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};

const CARD_WIDTH: f32 = 1200.0;
const CARD_HEIGHT: f32 = 675.0;

// ---------------------------------------------------------------------------
// Shared Preview / Export Renderer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct PnlCardCanvas {
    text: PnlCardRenderText,
    pnl_color: Color,
    outcome_label: &'static str,
    font: Font,
}

impl PnlCardCanvas {
    pub(super) fn new(text: PnlCardRenderText, upnl: f64, pnl_color: Color) -> Self {
        let outcome_label = if upnl > f64::EPSILON {
            "IN PROFIT"
        } else if upnl < -f64::EPSILON {
            "IN LOSS"
        } else {
            "BREAK EVEN"
        };

        Self {
            text,
            pnl_color,
            outcome_label,
            font: crate::app_fonts::monospace_font(),
        }
    }

    pub(super) fn font(&self) -> Font {
        self.font
    }

    pub(super) fn ticker(&self) -> &str {
        &self.text.ticker
    }

    pub(super) fn draw(
        &self,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
    ) -> Vec<canvas::Geometry> {
        draw_pnl_card(self, renderer, theme, bounds)
    }
}

impl<Message> canvas::Program<Message> for PnlCardCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        self.draw(renderer, theme, bounds)
    }
}

#[derive(Debug, Clone, Copy)]
struct CardLayout {
    origin: Point,
    scale: f32,
}

impl CardLayout {
    fn new(bounds: Rectangle) -> Self {
        let scale = (bounds.width / CARD_WIDTH)
            .min(bounds.height / CARD_HEIGHT)
            .max(0.01);
        let content = Size::new(CARD_WIDTH * scale, CARD_HEIGHT * scale);

        Self {
            origin: Point::new(
                (bounds.width - content.width) / 2.0,
                (bounds.height - content.height) / 2.0,
            ),
            scale,
        }
    }

    fn point(self, x: f32, y: f32) -> Point {
        Point::new(
            self.origin.x + x * self.scale,
            self.origin.y + y * self.scale,
        )
    }

    fn size(self, width: f32, height: f32) -> Size {
        Size::new(width * self.scale, height * self.scale)
    }

    fn pixels(self, value: f32) -> f32 {
        value * self.scale
    }
}

fn draw_pnl_card(
    card: &PnlCardCanvas,
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
) -> Vec<canvas::Geometry> {
    let mut frame = canvas::Frame::new(renderer, bounds.size());
    let layout = CardLayout::new(bounds);
    let palette = pnl_card_palette(theme, card.pnl_color);

    draw_surfaces(&mut frame, bounds, layout, palette);
    draw_logo(&mut frame, layout, palette);
    draw_text(&mut frame, layout, palette, card);

    vec![frame.into_geometry()]
}

fn draw_surfaces(
    frame: &mut canvas::Frame<Renderer>,
    bounds: Rectangle,
    layout: CardLayout,
    palette: PnlCardPalette,
) {
    frame.fill_rectangle(Point::ORIGIN, bounds.size(), palette.end);

    let inset = layout.pixels(2.0);
    let card_path = canvas::Path::rounded_rectangle(
        Point::new(inset, inset),
        Size::new(
            (bounds.width - inset * 2.0).max(0.0),
            (bounds.height - inset * 2.0).max(0.0),
        ),
        layout.pixels(28.0).into(),
    );
    let background =
        canvas::gradient::Linear::new(Point::ORIGIN, Point::new(bounds.width, bounds.height))
            .add_stop(0.0, palette.start)
            .add_stop(0.48, palette.mid)
            .add_stop(1.0, palette.end);
    frame.fill(&card_path, background);
    frame.stroke(
        &card_path,
        canvas::Stroke::default()
            .with_color(palette.border)
            .with_width(layout.pixels(2.0)),
    );

    let accent_rule = canvas::Path::rounded_rectangle(
        layout.point(52.0, 34.0),
        layout.size(152.0, 6.0),
        layout.pixels(3.0).into(),
    );
    frame.fill(&accent_rule, palette.accent);

    let glow = canvas::Path::circle(layout.point(997.0, 180.0), layout.pixels(142.0));
    frame.fill(&glow, with_alpha(palette.accent, 0.075));
    let inner_glow = canvas::Path::circle(layout.point(997.0, 180.0), layout.pixels(82.0));
    frame.fill(&inner_glow, with_alpha(palette.accent, 0.055));

    for y in [150.0, 205.0, 260.0, 315.0] {
        let guide = canvas::Path::line(layout.point(690.0, y), layout.point(1148.0, y));
        frame.stroke(
            &guide,
            canvas::Stroke::default()
                .with_color(with_alpha(palette.text, 0.045))
                .with_width(layout.pixels(1.0)),
        );
    }

    let metric_panel = canvas::Path::rounded_rectangle(
        layout.point(48.0, 482.0),
        layout.size(1104.0, 132.0),
        layout.pixels(18.0).into(),
    );
    frame.fill(&metric_panel, palette.panel);
    frame.stroke(
        &metric_panel,
        canvas::Stroke::default()
            .with_color(palette.panel_border)
            .with_width(layout.pixels(1.0)),
    );

    for x in [416.0, 784.0] {
        let divider = canvas::Path::line(layout.point(x, 508.0), layout.point(x, 588.0));
        frame.stroke(
            &divider,
            canvas::Stroke::default()
                .with_color(with_alpha(palette.text, 0.12))
                .with_width(layout.pixels(1.0)),
        );
    }

    let status_dot = canvas::Path::circle(layout.point(58.0, 644.0), layout.pixels(4.0));
    frame.fill(&status_dot, palette.accent);
}

fn draw_logo(frame: &mut canvas::Frame<Renderer>, layout: CardLayout, palette: PnlCardPalette) {
    let mark = canvas::Path::rounded_rectangle(
        layout.point(52.0, 66.0),
        layout.size(54.0, 54.0),
        layout.pixels(14.0).into(),
    );
    frame.fill(&mark, palette.brand);

    let k = canvas::Path::new(|path| {
        path.move_to(layout.point(72.0, 80.0));
        path.line_to(layout.point(72.0, 106.0));
        path.move_to(layout.point(70.0, 94.0));
        path.line_to(layout.point(91.0, 80.0));
        path.move_to(layout.point(70.0, 94.0));
        path.line_to(layout.point(91.0, 106.0));
    });
    frame.stroke(
        &k,
        canvas::Stroke {
            style: canvas::Style::Solid(palette.brand_ink),
            width: layout.pixels(5.5),
            line_cap: canvas::LineCap::Round,
            line_join: canvas::LineJoin::Round,
            ..Default::default()
        },
    );
}

fn draw_text(
    frame: &mut canvas::Frame<Renderer>,
    layout: CardLayout,
    palette: PnlCardPalette,
    card: &PnlCardCanvas,
) {
    let regular = card.font;
    let medium = Font {
        weight: Weight::Medium,
        ..regular
    };
    let bold = Font {
        weight: Weight::Bold,
        ..regular
    };

    fill_text(
        frame,
        "KEROSENE",
        layout.point(126.0, 70.0),
        layout.pixels(30.0),
        palette.text,
        bold,
        alignment::Horizontal::Left,
    );
    fill_text(
        frame,
        "TRADING TERMINAL",
        layout.point(127.0, 104.0),
        layout.pixels(18.0),
        palette.weak_text,
        medium,
        alignment::Horizontal::Left,
    );
    fill_text(
        frame,
        card.text.ticker.to_uppercase(),
        layout.point(1148.0, 66.0),
        layout.pixels(54.0),
        palette.text,
        bold,
        alignment::Horizontal::Right,
    );

    let badge_origin = layout.point(52.0, 188.0);
    let badge_size = layout.size(156.0, 42.0);
    let badge =
        canvas::Path::rounded_rectangle(badge_origin, badge_size, layout.pixels(21.0).into());
    frame.fill(&badge, with_alpha(palette.accent, 0.14));
    frame.stroke(
        &badge,
        canvas::Stroke::default()
            .with_color(with_alpha(palette.accent, 0.48))
            .with_width(layout.pixels(1.0)),
    );
    fill_text_centered(
        frame,
        card.outcome_label,
        Point::new(
            badge_origin.x + badge_size.width / 2.0,
            badge_origin.y + badge_size.height / 2.0,
        ),
        layout.pixels(21.0),
        palette.accent,
        bold,
    );

    fill_text(
        frame,
        format!(
            "UNREALIZED P&L  /  {}",
            card.text.percent_mode_label.to_uppercase()
        ),
        layout.point(230.0, 197.0),
        layout.pixels(22.0),
        palette.weak_text,
        medium,
        alignment::Horizontal::Left,
    );
    fill_text(
        frame,
        &card.text.primary_value,
        layout.point(50.0, 245.0),
        layout.pixels(108.0),
        palette.text,
        bold,
        alignment::Horizontal::Left,
    );
    if let Some(secondary) = &card.text.secondary_value {
        fill_text(
            frame,
            secondary,
            layout.point(56.0, 377.0),
            layout.pixels(38.0),
            palette.weak_text,
            medium,
            alignment::Horizontal::Left,
        );
    }

    draw_metric(
        frame,
        layout,
        palette,
        regular,
        bold,
        76.0,
        "LEVERAGE",
        &card.text.leverage_display,
    );
    draw_metric(
        frame,
        layout,
        palette,
        regular,
        bold,
        444.0,
        "ENTRY PRICE",
        &card.text.entry_display,
    );
    draw_metric(
        frame,
        layout,
        palette,
        regular,
        bold,
        812.0,
        "MARK PRICE",
        &card.text.exit_display,
    );

    fill_text(
        frame,
        card.text.context.to_uppercase(),
        layout.point(74.0, 631.0),
        layout.pixels(20.0),
        palette.weak_text,
        medium,
        alignment::Horizontal::Left,
    );
    fill_text(
        frame,
        "LIVE  /  MARK-TO-MARKET",
        layout.point(1148.0, 631.0),
        layout.pixels(20.0),
        palette.weak_text,
        medium,
        alignment::Horizontal::Right,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_metric(
    frame: &mut canvas::Frame<Renderer>,
    layout: CardLayout,
    palette: PnlCardPalette,
    regular: Font,
    bold: Font,
    x: f32,
    label: &str,
    value: &str,
) {
    fill_text(
        frame,
        label,
        layout.point(x, 510.0),
        layout.pixels(20.0),
        palette.weak_text,
        regular,
        alignment::Horizontal::Left,
    );
    fill_text(
        frame,
        value,
        layout.point(x, 550.0),
        layout.pixels(34.0),
        palette.text,
        bold,
        alignment::Horizontal::Left,
    );
}

fn fill_text(
    frame: &mut canvas::Frame<Renderer>,
    content: impl Into<String>,
    position: Point,
    size: f32,
    color: Color,
    font: Font,
    align_x: alignment::Horizontal,
) {
    frame.fill_text(canvas::Text {
        content: content.into(),
        position,
        color,
        size: Pixels(size),
        font,
        align_x: align_x.into(),
        align_y: alignment::Vertical::Top,
        ..Default::default()
    });
}

fn fill_text_centered(
    frame: &mut canvas::Frame<Renderer>,
    content: impl Into<String>,
    position: Point,
    size: f32,
    color: Color,
    font: Font,
) {
    frame.fill_text(canvas::Text {
        content: content.into(),
        position,
        color,
        size: Pixels(size),
        font,
        align_x: alignment::Horizontal::Center.into(),
        align_y: alignment::Vertical::Center,
        ..Default::default()
    });
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_layout_preserves_the_export_aspect_ratio() {
        let preview = CardLayout::new(Rectangle::with_size(Size::new(480.0, 270.0)));
        let export = CardLayout::new(Rectangle::with_size(Size::new(1200.0, 675.0)));

        assert!((preview.scale - 0.4).abs() < f32::EPSILON);
        assert!((export.scale - 1.0).abs() < f32::EPSILON);
        assert_eq!(preview.origin, Point::ORIGIN);
        assert_eq!(export.origin, Point::ORIGIN);
    }
}
