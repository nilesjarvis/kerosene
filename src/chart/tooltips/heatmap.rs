use super::{TooltipLine, TooltipSurface};
use crate::chart::formatting::format_compact_coins;
use crate::denomination::format_compact_usd;
use crate::helpers::format_price;
use crate::hyperdash_api::HeatmapRect;
use iced::{Color, Point};

impl TooltipSurface<'_> {
    pub(in crate::chart) fn draw_heatmap_hover<X, Y>(
        &mut self,
        rects: &[HeatmapRect],
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
        for rect in rects {
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
                    "{} - {}",
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
                    "{}: {} ({})",
                    side,
                    format_compact_coins(coins.abs()),
                    format_compact_usd(usd),
                ),
                color: side_color,
            },
        ];

        let card_size = Self::card_size_for_lines(&lines, 170.0);
        let card_x = (self.pos.x + 14.0)
            .min(self.chart_w - card_size.width - 4.0)
            .max(4.0);
        let max_card_y = (self.price_h - card_size.height).max(0.0);
        let card_y = (self.pos.y - card_size.height - 8.0).clamp(0.0, max_card_y);

        self.draw_card(Point::new(card_x, card_y), card_size, &lines);
    }
}
