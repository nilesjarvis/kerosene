use super::CandleLayerContext;
use crate::chart::model::{
    CandlestickChart, FUNDING_MODE_BUTTON_HEIGHT, FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_X,
    FUNDING_MODE_BUTTON_Y_OFFSET, FUNDING_RATE_ANNUALIZATION_FACTOR,
};
use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Funding Rate Panel Rendering
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_funding_panel<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        if ctx.funding_panel_h <= 0.0 {
            return;
        }

        let panel_y = ctx.chart_h;
        frame.fill_rectangle(
            Point::new(0.0, panel_y),
            Size::new(ctx.chart_w, ctx.funding_panel_h),
            Color {
                a: 0.025,
                ..ctx.theme.palette().text
            },
        );

        let separator =
            canvas::Path::line(Point::new(0.0, panel_y), Point::new(ctx.chart_w, panel_y));
        frame.stroke(
            &separator,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.14,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
        for offset in [-8.0_f32, 0.0, 8.0] {
            frame.fill_rectangle(
                Point::new(ctx.chart_w * 0.5 + offset - 2.5, panel_y + 2.0),
                Size::new(5.0, 1.0),
                Color {
                    a: 0.28,
                    ..ctx.theme.palette().text
                },
            );
        }
        self.draw_funding_mode_button(ctx, frame, panel_y);

        let plot_top = panel_y + 24.0;
        let plot_bottom = panel_y + ctx.funding_panel_h - 8.0;
        if plot_bottom <= plot_top {
            return;
        }

        let baseline_y = (plot_top + plot_bottom) * 0.5;
        let baseline = canvas::Path::line(
            Point::new(0.0, baseline_y),
            Point::new(ctx.chart_w, baseline_y),
        );
        frame.stroke(
            &baseline,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.10,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );

        let visible: Vec<_> = self
            .funding_rates
            .iter()
            .filter_map(|point| {
                let x = self.timestamp_to_x(point.time_ms, ctx.state, ctx.chart_w)?;
                (x >= -ctx.step && x <= ctx.chart_w + ctx.step).then_some((x, point))
            })
            .collect();

        if visible.is_empty() {
            if self.funding_rates.is_empty() {
                self.draw_funding_status(ctx, frame, panel_y);
            } else {
                self.draw_funding_message(ctx, frame, panel_y, "No funding in view", false);
            }
            return;
        }

        let max_abs = visible
            .iter()
            .map(|(_, point)| self.display_funding_rate(point.rate).abs())
            .fold(0.0_f64, f64::max);
        if max_abs <= 0.0 {
            self.draw_funding_message(ctx, frame, panel_y, "Funding flat", false);
            return;
        }

        let scaled_max = (max_abs * 1.12 * ctx.state.funding_y_scale).max(f64::EPSILON);
        let half_h = (plot_bottom - plot_top) * 0.5;
        let bar_w = (ctx.step * 0.34).clamp(1.0, 4.0);
        let stride = (visible.len() / 2_000).max(1);

        for (x, point) in visible.iter().step_by(stride).copied() {
            let display_rate = self.display_funding_rate(point.rate);
            let offset = (display_rate / scaled_max) as f32 * half_h;
            let y = (baseline_y - offset).clamp(plot_top, plot_bottom);
            let top = y.min(baseline_y);
            let height = (baseline_y - y).abs().max(1.0);
            let color = if display_rate >= 0.0 {
                ctx.candle_bull_color
            } else {
                ctx.candle_bear_color
            };
            frame.fill_rectangle(
                Point::new(x - bar_w * 0.5, top),
                Size::new(bar_w, height),
                Color { a: 0.78, ..color },
            );
        }

        self.draw_funding_axis_label(ctx, frame, plot_top, scaled_max);
        self.draw_funding_axis_label(ctx, frame, baseline_y, 0.0);
        self.draw_funding_axis_label(ctx, frame, plot_bottom, -scaled_max);
    }

    fn draw_funding_mode_button<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let origin = Point::new(
            FUNDING_MODE_BUTTON_X,
            panel_y + FUNDING_MODE_BUTTON_Y_OFFSET,
        );
        let size = Size::new(FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_HEIGHT);
        let bg = if self.funding_annualized {
            Color {
                a: 0.20,
                ..ctx.theme.palette().primary
            }
        } else {
            Color {
                a: 0.10,
                ..ctx.theme.palette().text
            }
        };
        frame.fill_rectangle(origin, size, bg);
        let border = canvas::Path::rectangle(origin, size);
        frame.stroke(
            &border,
            canvas::Stroke::default()
                .with_color(Color {
                    a: 0.18,
                    ..ctx.theme.palette().text
                })
                .with_width(1.0),
        );
        frame.fill_text(canvas::Text {
            content: if self.funding_annualized {
                "APR".to_string()
            } else {
                "1H".to_string()
            },
            position: Point::new(origin.x + size.width * 0.5, origin.y + size.height * 0.5),
            color: Color {
                a: 0.82,
                ..ctx.theme.palette().text
            },
            size: iced::Pixels(9.0),
            align_x: alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    fn draw_funding_status<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let (label, is_error) = self
            .funding_status
            .as_ref()
            .map(|(label, is_error)| (label.as_str(), *is_error))
            .unwrap_or(("Funding waiting for data", false));
        self.draw_funding_message(ctx, frame, panel_y, label, is_error);
    }

    fn draw_funding_message<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        panel_y: f32,
        label: &str,
        is_error: bool,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        let color = if is_error {
            ctx.theme.palette().danger
        } else {
            Color {
                a: 0.45,
                ..ctx.theme.palette().text
            }
        };
        frame.fill_text(canvas::Text {
            content: label.to_string(),
            position: Point::new(
                FUNDING_MODE_BUTTON_X + FUNDING_MODE_BUTTON_WIDTH + 8.0,
                panel_y + ctx.funding_panel_h * 0.5,
            ),
            color,
            size: iced::Pixels(10.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    fn draw_funding_axis_label<IdxToCx, PriceToY>(
        &self,
        ctx: &CandleLayerContext<'_, IdxToCx, PriceToY>,
        frame: &mut canvas::Frame,
        y: f32,
        rate: f64,
    ) where
        IdxToCx: Fn(usize) -> f32,
        PriceToY: Fn(f64) -> f32,
    {
        frame.fill_text(canvas::Text {
            content: format_funding_rate_percent(rate, self.funding_annualized),
            position: Point::new(ctx.chart_w + 6.0, y),
            color: Color {
                a: 0.42,
                ..ctx.theme.palette().text
            },
            size: iced::Pixels(9.0),
            align_x: alignment::Horizontal::Left.into(),
            align_y: alignment::Vertical::Center,
            font: iced::Font::MONOSPACE,
            ..canvas::Text::default()
        });
    }

    pub(in crate::chart) fn display_funding_rate(&self, hourly_rate: f64) -> f64 {
        if self.funding_annualized {
            hourly_rate * FUNDING_RATE_ANNUALIZATION_FACTOR
        } else {
            hourly_rate
        }
    }
}

pub(in crate::chart) fn format_funding_rate_percent(rate: f64, annualized: bool) -> String {
    if annualized {
        format!("{:+.2}%", rate * 100.0)
    } else {
        format!("{:+.5}%", rate * 100.0)
    }
}
