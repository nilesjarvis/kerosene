use super::{TooltipLine, TooltipSurface};
use crate::chart::formatting::{format_compact, format_compact_coins};
use crate::helpers::format_price;
use crate::hyperdash_api::HeatmapRect;
use iced::{Color, Point, Size};

impl TooltipSurface<'_> {
    pub(in crate::chart) fn draw_heatmap_hover<X, Y>(
        &mut self,
        rects: &[HeatmapRect],
        stride: usize,
        max_usd: f64,
        mut rect_x_bounds: X,
        price_to_y: &Y,
    ) where
        X: FnMut(&HeatmapRect) -> Option<(f32, f32)>,
        Y: Fn(f64) -> f32,
    {
        if rects.is_empty() || max_usd <= 0.0 {
            return;
        }

        let mut best: Option<&HeatmapRect> = None;
        let mut best_dist = f32::INFINITY;
        for rect in rects.iter().step_by(stride) {
            let Some((x_left, x_right)) = rect_x_bounds(rect) else {
                continue;
            };
            if self.pos.x < x_left || self.pos.x > x_right {
                continue;
            }

            let y_top = price_to_y(rect.price_hi);
            let y_bot = price_to_y(rect.price_lo);
            if self.pos.y < y_top || self.pos.y > y_bot {
                continue;
            }

            let cx = (x_left + x_right) * 0.5;
            let dx = (self.pos.x - cx).abs();
            if dx < best_dist {
                best_dist = dx;
                best = Some(rect);
            }
        }

        let Some(rect) = best else {
            return;
        };

        let coins = rect.amount_coins;
        let usd = rect.amount_usd.abs();
        let side = if coins >= 0.0 { "Longs" } else { "Shorts" };
        let side_color = if coins >= 0.0 {
            self.theme.palette().success
        } else {
            self.theme.palette().danger
        };

        let lines = vec![
            TooltipLine {
                content: format!(
                    "${} - ${}",
                    format_price(rect.price_lo),
                    format_price(rect.price_hi)
                ),
                color: Color {
                    a: 0.7,
                    ..self.theme.palette().text
                },
            },
            TooltipLine {
                content: format!(
                    "{}: {} (${})",
                    side,
                    format_compact_coins(coins.abs()),
                    format_compact(usd),
                ),
                color: side_color,
            },
        ];

        let line_h: f32 = 14.0;
        let pad: f32 = 6.0;
        let card_w: f32 = 170.0;
        let card_h = lines.len() as f32 * line_h + pad * 2.0;
        let card_x = (self.pos.x + 14.0)
            .min(self.chart_w - card_w - 4.0)
            .max(4.0);
        let card_y = (self.pos.y - card_h - 8.0).clamp(0.0, self.price_h - card_h);

        self.draw_card(
            Point::new(card_x, card_y),
            Size::new(card_w, card_h),
            &lines,
        );
    }
}
