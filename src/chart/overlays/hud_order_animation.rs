use super::TradingOverlayContext;
use crate::chart::model::CandlestickChart;
use iced::widget::canvas;
use iced::{Color, Point};

// ---------------------------------------------------------------------------
// HUD Order Placement Animation
// ---------------------------------------------------------------------------

const DOT_SPACING: f32 = 18.0;
const DOT_RADIUS: f32 = 1.7;

impl CandlestickChart {
    pub(super) fn draw_market_order_loading<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if self.pending_market_order_loading.is_empty() || ctx.chart_w <= 0.0 || ctx.price_h <= 0.0
        {
            return;
        }

        let center = Point::new(ctx.chart_w * 0.5, ctx.price_h * 0.5);
        for (index, loader) in self.pending_market_order_loading.iter().enumerate() {
            let color = if loader.is_buy {
                ctx.candle_bull_color
            } else {
                ctx.candle_bear_color
            };
            let phase = (loader.progress + index as f32 * 0.21).fract();
            draw_dot_wave(ctx, center, color, phase, DotWaveStyle::market_loading());
            draw_dot_wave(
                ctx,
                center,
                color,
                (phase + 0.5).fract(),
                DotWaveStyle::market_loading(),
            );
        }
    }

    pub(super) fn draw_hud_order_animation<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        let Some(animation) = self.hud_order_animation else {
            return;
        };
        if ctx.price_range <= 0.0 {
            return;
        }

        let progress = animation.progress.clamp(0.0, 1.0);
        let eased = ease_out_cubic(progress);
        let color = if animation.is_buy {
            ctx.candle_bull_color
        } else {
            ctx.candle_bear_color
        };
        let origin_x = animation.origin_x.clamp(0.0, ctx.chart_w);

        let center = if animation.show_line {
            let y = (ctx.price_to_y)(animation.price);
            if y < -24.0 || y > ctx.price_h + 24.0 {
                return;
            }

            let alpha = (1.0 - progress * 0.55).clamp(0.0, 1.0);
            let left_x = origin_x - origin_x * eased;
            let right_x = origin_x + (ctx.chart_w - origin_x) * eased;
            let stroke = canvas::Stroke::default()
                .with_color(Color {
                    a: 0.82 * alpha,
                    ..color
                })
                .with_width(1.4 + 1.4 * (1.0 - progress))
                .with_line_cap(canvas::LineCap::Round);

            ctx.fisheye.stroke_projected_line(
                ctx.frame,
                Point::new(left_x, y),
                Point::new(origin_x, y),
                stroke,
            );
            ctx.fisheye.stroke_projected_line(
                ctx.frame,
                Point::new(origin_x, y),
                Point::new(right_x, y),
                stroke,
            );
            Point::new(origin_x, y)
        } else {
            Point::new(
                origin_x,
                animation
                    .click_y
                    .clamp(0.0, animation.chart_h.min(ctx.price_h)),
            )
        };

        draw_dot_wave(ctx, center, color, progress, DotWaveStyle::hud_submit());
    }
}

#[derive(Debug, Clone, Copy)]
struct DotWaveStyle {
    radius_start: f32,
    radius_span: f32,
    thickness: f32,
    alpha: f32,
    dot_radius: f32,
}

impl DotWaveStyle {
    fn hud_submit() -> Self {
        Self {
            radius_start: 18.0,
            radius_span: 145.0,
            thickness: 24.0,
            alpha: 0.58,
            dot_radius: DOT_RADIUS,
        }
    }

    fn market_loading() -> Self {
        Self {
            radius_start: 26.0,
            radius_span: 210.0,
            thickness: 34.0,
            alpha: 0.16,
            dot_radius: 1.35,
        }
    }
}

fn draw_dot_wave<PriceToY, IdxToCx>(
    ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    center: Point,
    color: Color,
    progress: f32,
    style: DotWaveStyle,
) where
    PriceToY: Fn(f64) -> f32,
    IdxToCx: Fn(usize) -> f32,
{
    let radius = style.radius_start + style.radius_span * ease_out_cubic(progress);
    let thickness = style.thickness;
    let fade = (1.0 - progress).clamp(0.0, 1.0);
    let x_min = (center.x - radius - thickness).max(0.0);
    let x_max = (center.x + radius + thickness).min(ctx.chart_w);
    let y_min = (center.y - radius - thickness).max(0.0);
    let y_max = (center.y + radius + thickness).min(ctx.price_h);

    let mut y = (y_min / DOT_SPACING).floor() * DOT_SPACING + DOT_SPACING * 0.5;
    while y <= y_max {
        let mut x = (x_min / DOT_SPACING).floor() * DOT_SPACING + DOT_SPACING * 0.5;
        while x <= x_max {
            let dx = x - center.x;
            let dy = y - center.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let wave = 1.0 - ((distance - radius).abs() / thickness).clamp(0.0, 1.0);
            if wave > 0.0 {
                ctx.fisheye.fill_projected_circle(
                    ctx.frame,
                    Point::new(x, y),
                    style.dot_radius + wave * 1.6,
                    Color {
                        a: wave * fade * style.alpha,
                        ..color
                    },
                );
            }
            x += DOT_SPACING;
        }
        y += DOT_SPACING;
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
