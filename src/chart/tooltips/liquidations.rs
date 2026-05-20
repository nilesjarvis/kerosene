use super::{TooltipLine, TooltipSurface};
use crate::chart::formatting::format_compact_coins;
use crate::denomination::DisplayDenominationContext;
use crate::hyperdash_api::LiquidationBucket;
use iced::{Color, Point};

impl TooltipSurface<'_> {
    pub(in crate::chart) fn draw_liquidation_hover<F>(
        &mut self,
        hover_price: f64,
        price_range: f64,
        buckets: &[LiquidationBucket],
        price_to_y: &F,
        denomination: &DisplayDenominationContext,
    ) where
        F: Fn(f64) -> f32,
    {
        if buckets.is_empty() {
            return;
        }

        let mut best_idx = 0;
        let mut best_dist = f64::INFINITY;
        for (index, bucket) in buckets.iter().enumerate() {
            let dist = (bucket.price_center - hover_price).abs();
            if dist < best_dist {
                best_dist = dist;
                best_idx = index;
            }
        }

        let bucket = &buckets[best_idx];
        let bucket_count = buckets.len();
        let bucket_px_h = self.price_h / bucket_count as f32;
        let bucket_y = price_to_y(bucket.price_center);
        if (self.pos.y - bucket_y).abs() > bucket_px_h * 0.6
            || (bucket.long_usd <= 0.0 && bucket.short_usd <= 0.0)
        {
            return;
        }

        let half_w = (price_range / bucket_count as f64) * 0.5;
        let lo = bucket.price_center - half_w;
        let hi = bucket.price_center + half_w;
        let mut lines = vec![TooltipLine {
            content: format!(
                "{} - {}",
                denomination.format_chart_price(lo),
                denomination.format_chart_price(hi)
            ),
            color: Color {
                a: 0.7,
                ..self.theme.palette().text
            },
        }];

        if bucket.long_usd > 0.0 {
            lines.push(TooltipLine {
                content: format!(
                    "Longs: {} ({})  ",
                    format_compact_coins(bucket.long_coins),
                    denomination.format_compact_value(bucket.long_usd),
                ),
                color: self.theme.palette().success,
            });
        }
        if bucket.short_usd > 0.0 {
            lines.push(TooltipLine {
                content: format!(
                    "Shorts: {} ({})  ",
                    format_compact_coins(bucket.short_coins),
                    denomination.format_compact_value(bucket.short_usd),
                ),
                color: self.theme.palette().danger,
            });
        }
        let total_usd = bucket.long_usd + bucket.short_usd;
        if total_usd > 0.0 && bucket.long_usd > 0.0 && bucket.short_usd > 0.0 {
            lines.push(TooltipLine {
                content: format!("Total: {}", denomination.format_compact_value(total_usd)),
                color: Color::WHITE,
            });
        }

        let card_size = Self::card_size_for_lines(&lines, 170.0);
        let card_x = (self.pos.x - card_size.width - 12.0).max(4.0);
        let max_card_y = (self.price_h - card_size.height).max(0.0);
        let card_y = (self.pos.y - card_size.height * 0.5).clamp(0.0, max_card_y);

        self.draw_card(Point::new(card_x, card_y), card_size, &lines);
    }
}
