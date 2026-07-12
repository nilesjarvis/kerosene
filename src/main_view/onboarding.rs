use crate::app_state::TradingTerminal;
use crate::helpers::text_color_for_bg;
use crate::message::Message;
use iced::widget::canvas;
use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, stack, text};
use iced::{
    Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme, mouse,
};

// ---------------------------------------------------------------------------
// App Onboarding
// ---------------------------------------------------------------------------

const ONBOARDING_CONTENT_WIDTH: f32 = 720.0;
const ONBOARDING_GRAPHIC_WIDTH: f32 = 260.0;
const ONBOARDING_GRAPHIC_HEIGHT: f32 = 176.0;

// The animation phase wraps at a large multiple of TAU rather than at TAU: every
// sine/cosine term below uses a phase coefficient that is an integer multiple of
// 0.01, so 100*TAU lands each term on a whole period at the wrap (seamless loop),
// while staying small enough to keep f32 precision crisp. The two scrolling terms
// (grid offset, price-path start) are explicitly tied to this period so they wrap
// seamlessly too. See `advance_onboarding_phase`.
const ONBOARDING_PHASE_PERIOD: f32 = 100.0 * std::f32::consts::TAU;
const ONBOARDING_PHASE_STEP: f32 = 0.35;

struct OnboardingBackdrop {
    phase: f32,
}

struct OnboardingGraphic {
    phase: f32,
}

impl canvas::Program<Message> for OnboardingBackdrop {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let palette = theme.palette();
        let extended = theme.extended_palette();
        let base = extended.background.base.color;
        let strong = extended.background.strong.color;

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), mix_color(base, strong, 0.28));

        draw_grid(&mut frame, bounds.size(), self.phase, palette.primary);
        draw_price_paths(
            &mut frame,
            bounds.size(),
            self.phase,
            palette.success,
            palette.danger,
        );
        draw_market_bars(&mut frame, bounds.size(), self.phase, palette.primary);

        vec![frame.into_geometry()]
    }
}

impl canvas::Program<Message> for OnboardingGraphic {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let palette = theme.palette();
        let extended = theme.extended_palette();
        let w = bounds.width;
        let h = bounds.height;
        let panel = Color {
            a: 0.54,
            ..extended.background.weak.color
        };
        let border = Color {
            a: 0.34,
            ..palette.primary
        };

        let terminal = canvas::Path::new(|p| {
            p.move_to(Point::new(w * 0.06, h * 0.08));
            p.line_to(Point::new(w * 0.88, h * 0.08));
            p.line_to(Point::new(w * 0.96, h * 0.22));
            p.line_to(Point::new(w * 0.96, h * 0.82));
            p.line_to(Point::new(w * 0.88, h * 0.94));
            p.line_to(Point::new(w * 0.06, h * 0.94));
            p.close();
        });
        frame.fill(&terminal, panel);
        frame.stroke(
            &terminal,
            canvas::Stroke::default().with_color(border).with_width(1.2),
        );

        let header = canvas::Path::line(
            Point::new(w * 0.08, h * 0.24),
            Point::new(w * 0.94, h * 0.24),
        );
        frame.stroke(
            &header,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.24,
                    ..palette.text
                })
                .with_width(1.0),
        );

        draw_terminal_controls(
            &mut frame,
            w,
            h,
            palette.primary,
            palette.success,
            palette.danger,
        );
        draw_terminal_candles(
            &mut frame,
            w,
            h,
            self.phase,
            palette.success,
            palette.danger,
        );
        draw_terminal_depth(
            &mut frame,
            w,
            h,
            self.phase,
            palette.success,
            palette.danger,
        );

        vec![frame.into_geometry()]
    }
}

impl TradingTerminal {
    /// Advance the first-run onboarding animation. Unlike `spinner_phase` (an
    /// angle wrapped at TAU), this accumulates and wraps at `ONBOARDING_PHASE_PERIOD`
    /// so the looping welcome visuals never jump when the phase resets.
    pub(crate) fn advance_onboarding_phase(&mut self) {
        self.onboarding_phase =
            (self.onboarding_phase + ONBOARDING_PHASE_STEP).rem_euclid(ONBOARDING_PHASE_PERIOD);
    }

    pub(super) fn view_onboarding(&self) -> Element<'_, Message> {
        self.view_onboarding_body(None)
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(super) fn view_onboarding_with_top_bar<'a>(
        &'a self,
        top_bar: Element<'a, Message>,
    ) -> Element<'a, Message> {
        self.view_onboarding_body(Some(top_bar))
    }

    fn view_onboarding_body<'a>(
        &'a self,
        top_bar: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let backdrop = iced::widget::canvas(OnboardingBackdrop {
            phase: self.onboarding_phase,
        })
        .width(Fill)
        .height(Fill);

        let content = column![
            iced::widget::canvas(OnboardingGraphic {
                phase: self.onboarding_phase,
            })
            .width(Length::Fixed(ONBOARDING_GRAPHIC_WIDTH))
            .height(Length::Fixed(ONBOARDING_GRAPHIC_HEIGHT)),
            text("Kerosene").size(44).center(),
            text("A GPU-accelerated desktop trading terminal for Hyperliquid.")
                .size(15)
                .center(),
            row![
                market_chip("Live markets", |theme| theme.palette().primary),
                market_chip("Charting", |theme| theme.palette().success),
                market_chip("Automation", |theme| theme.palette().danger),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            button(text("Enter Terminal").size(14).center())
                .on_press(Message::EnterApplication)
                .padding([11, 24])
                .style(onboarding_button_style)
        ]
        .spacing(18)
        .width(Fill)
        .align_x(Alignment::Center);

        // Outer container fills the window and centers the width-capped content;
        // the inner container applies the max width. Collapsing both into a single
        // `max_width(..).center(..)` container leaves the content pinned to the left
        // edge on screens wider than ONBOARDING_CONTENT_WIDTH, because the stack
        // anchors each layer at the top-left rather than centering it.
        let content_layer = container(
            container(content)
                .width(Fill)
                .max_width(ONBOARDING_CONTENT_WIDTH)
                .padding([28, 24]),
        )
        .width(Fill)
        .height(Fill)
        .center(Fill);

        let body = container(stack![backdrop, content_layer].width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            });

        match top_bar {
            Some(top_bar) => column![top_bar, body].width(Fill).height(Fill).into(),
            None => body.into(),
        }
    }
}

fn market_chip<'a>(label: &'static str, color: fn(&Theme) -> Color) -> Element<'a, Message> {
    container(
        row![chip_dot(color), text(label).size(11)]
            .spacing(7)
            .align_y(Alignment::Center),
    )
    .padding([5, 9])
    .style(move |theme: &Theme| {
        let accent = color(theme);
        container_style::Style {
            background: Some(Color { a: 0.13, ..accent }.into()),
            text_color: Some(Color {
                a: 0.92,
                ..theme.palette().text
            }),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color { a: 0.26, ..accent },
            },
            ..Default::default()
        }
    })
    .into()
}

fn chip_dot<'a>(color: fn(&Theme) -> Color) -> Element<'a, Message> {
    container(Space::new().width(7).height(7))
        .style(move |theme: &Theme| {
            let accent = color(theme);
            container_style::Style {
                background: Some(Color { a: 0.8, ..accent }.into()),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn onboarding_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let base = theme.palette().primary;
    let bg = match status {
        button::Status::Hovered => mix_color(base, theme.palette().success, 0.18),
        button::Status::Pressed => mix_color(base, theme.palette().text, 0.10),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: text_color_for_bg(bg),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color {
                a: 0.42,
                ..theme.palette().text
            },
        },
        ..Default::default()
    }
}

fn draw_grid(frame: &mut canvas::Frame, size: Size, phase: f32, accent: Color) {
    let spacing = 34.0;
    // Scroll a whole number of cells per phase period so the offset returns to 0 at
    // the wrap; `x_cells`/`y_cells` set the (parallax) scroll speed (~8 and ~5.2
    // px per phase unit, matching the original feel).
    let cycle = phase / ONBOARDING_PHASE_PERIOD;
    let x_cells = (8.0 * ONBOARDING_PHASE_PERIOD / spacing).round();
    let y_cells = (5.2 * ONBOARDING_PHASE_PERIOD / spacing).round();
    let offset_x = (cycle * x_cells * spacing).rem_euclid(spacing);
    let offset_y = (cycle * y_cells * spacing).rem_euclid(spacing);
    let grid_color = Color { a: 0.08, ..accent };

    // Batch every grid line into a single path + stroke rather than one path and
    // stroke call per line.
    let grid = canvas::Path::new(|p| {
        let mut x = -spacing + offset_x;
        while x <= size.width + spacing {
            p.move_to(Point::new(x, 0.0));
            p.line_to(Point::new(x, size.height));
            x += spacing;
        }

        let mut y = -spacing + offset_y;
        while y <= size.height + spacing {
            p.move_to(Point::new(0.0, y));
            p.line_to(Point::new(size.width, y));
            y += spacing;
        }
    });
    frame.stroke(
        &grid,
        canvas::Stroke::default()
            .with_color(grid_color)
            .with_width(1.0),
    );
}

fn draw_price_paths(
    frame: &mut canvas::Frame,
    size: Size,
    phase: f32,
    success: Color,
    danger: Color,
) {
    // Tie the per-lane scroll to whole cycles of the phase period so the path
    // start position wraps seamlessly along with the phase.
    let cycle = phase / ONBOARDING_PHASE_PERIOD;
    let path_cycles = (0.12 * ONBOARDING_PHASE_PERIOD).round();
    for lane in 0..3 {
        let progress = (cycle * path_cycles + lane as f32 * 0.27).fract();
        let y_base = size.height * (0.22 + lane as f32 * 0.22);
        let amp = 24.0 + lane as f32 * 8.0;
        let start_x = -size.width * progress;
        let color = if lane % 2 == 0 { success } else { danger };
        let path = canvas::Path::new(|p| {
            let mut x = start_x;
            let mut first = true;
            while x < size.width + 120.0 {
                let t = (x / 82.0) + phase + lane as f32;
                let point = Point::new(x, y_base + t.sin() * amp + (t * 0.43).cos() * 9.0);
                if first {
                    p.move_to(point);
                    first = false;
                } else {
                    p.line_to(point);
                }
                x += 54.0;
            }
        });
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(Color { a: 0.18, ..color })
                .with_width(1.4),
        );
    }
}

fn draw_market_bars(frame: &mut canvas::Frame, size: Size, phase: f32, accent: Color) {
    let bar_count = 28;
    let bar_w = (size.width / bar_count as f32).max(10.0);
    for i in 0..bar_count {
        let t = phase + i as f32 * 0.56;
        let height = 18.0 + (t.sin() * 0.5 + 0.5) * 68.0;
        let x = i as f32 * bar_w;
        let y = size.height - height - 18.0;
        frame.fill_rectangle(
            Point::new(x, y),
            Size::new((bar_w * 0.46).max(3.0), height),
            Color { a: 0.055, ..accent },
        );
    }
}

fn draw_terminal_controls(
    frame: &mut canvas::Frame,
    w: f32,
    h: f32,
    primary: Color,
    success: Color,
    danger: Color,
) {
    let controls = [(w * 0.11, danger), (w * 0.17, primary), (w * 0.23, success)];
    for (x, color) in controls {
        frame.fill(
            &canvas::Path::circle(Point::new(x, h * 0.16), 4.0),
            Color { a: 0.72, ..color },
        );
    }
}

fn draw_terminal_candles(
    frame: &mut canvas::Frame,
    w: f32,
    h: f32,
    phase: f32,
    success: Color,
    danger: Color,
) {
    let plot_left = w * 0.10;
    let plot_top = h * 0.34;
    let plot_h = h * 0.40;
    let candle_w = w * 0.035;
    for i in 0..10 {
        let x = plot_left + i as f32 * w * 0.055;
        let t = phase * 0.8 + i as f32 * 0.7;
        let center = plot_top + plot_h * (0.50 + t.sin() * 0.26);
        let body_h = 12.0 + (t * 1.7).cos().abs() * 24.0;
        let wick_h = body_h + 16.0 + (t * 0.9).sin().abs() * 16.0;
        let color = if i % 3 == 1 { danger } else { success };
        frame.stroke(
            &canvas::Path::line(
                Point::new(x + candle_w / 2.0, center - wick_h / 2.0),
                Point::new(x + candle_w / 2.0, center + wick_h / 2.0),
            ),
            canvas::Stroke::default()
                .with_color(Color { a: 0.56, ..color })
                .with_width(1.2),
        );
        frame.fill_rectangle(
            Point::new(x, center - body_h / 2.0),
            Size::new(candle_w, body_h),
            Color { a: 0.70, ..color },
        );
    }
}

fn draw_terminal_depth(
    frame: &mut canvas::Frame,
    w: f32,
    h: f32,
    phase: f32,
    success: Color,
    danger: Color,
) {
    let left = w * 0.68;
    let top = h * 0.34;
    let row_h = h * 0.07;
    for i in 0..5 {
        let ask_width = w * (0.12 + ((phase + i as f32) * 0.9).sin().abs() * 0.14);
        let bid_width = w * (0.11 + ((phase + i as f32) * 0.7).cos().abs() * 0.16);
        let y = top + i as f32 * row_h;
        frame.fill_rectangle(
            Point::new(left, y),
            Size::new(ask_width, row_h * 0.42),
            Color { a: 0.24, ..danger },
        );
        frame.fill_rectangle(
            Point::new(left, y + row_h * 0.50),
            Size::new(bid_width, row_h * 0.42),
            Color { a: 0.24, ..success },
        );
    }
}

fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onboarding_phase_advances_then_wraps_within_period() {
        let (mut terminal, _) = TradingTerminal::boot();
        assert_eq!(terminal.onboarding_phase, 0.0);

        terminal.advance_onboarding_phase();
        assert!((terminal.onboarding_phase - ONBOARDING_PHASE_STEP).abs() < 1e-4);

        // Drive well past the period; the phase must stay bounded and only ever move
        // forward by one step or wrap back toward zero.
        for _ in 0..5_000 {
            let before = terminal.onboarding_phase;
            terminal.advance_onboarding_phase();
            let after = terminal.onboarding_phase;
            assert!(
                (0.0..ONBOARDING_PHASE_PERIOD).contains(&after),
                "phase {after} escaped [0, period)"
            );
            let delta = after - before;
            assert!(
                (delta - ONBOARDING_PHASE_STEP).abs() < 1e-3 || delta < 0.0,
                "unexpected phase delta {delta}"
            );
        }
    }

    #[test]
    fn onboarding_phase_period_keeps_animation_terms_seamless() {
        // The period must be a whole multiple of TAU so every sine/cosine term in
        // the canvases lands on a full period when the phase wraps.
        let multiples = ONBOARDING_PHASE_PERIOD / std::f32::consts::TAU;
        assert!((multiples - multiples.round()).abs() < 1e-3);

        // Every non-unit phase coefficient used by the onboarding canvases must turn
        // into a whole number when scaled by that multiple, or the wrap would still
        // produce a visible jump. Update this list if a coefficient changes.
        for coeff in [0.8_f32, 0.72, 1.36, 0.9, 0.7, 0.43] {
            let scaled = coeff * multiples;
            assert!(
                (scaled - scaled.round()).abs() < 1e-2,
                "coefficient {coeff} is not seamless at the phase period"
            );
        }
    }
}
