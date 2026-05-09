use super::{TooltipLine, TooltipSurface};
use crate::chart::candle_layer::format_funding_rate_percent;
use crate::chart::model::FUNDING_RATE_ANNUALIZATION_FACTOR;
use crate::helpers::format_timestamp_exact;
use crate::hydromancer_api::FundingRatePoint;
use iced::{Color, Point, Size};

impl TooltipSurface<'_> {
    pub(in crate::chart) fn draw_funding_hover<X>(
        &mut self,
        points: &[FundingRatePoint],
        panel_y: f32,
        panel_h: f32,
        annualized: bool,
        point_to_x: X,
    ) where
        X: Fn(&FundingRatePoint) -> Option<f32>,
    {
        if points.is_empty() || panel_h <= 0.0 {
            return;
        }

        let mut best: Option<&FundingRatePoint> = None;
        let mut best_dist = f32::INFINITY;
        for point in points {
            let Some(x) = point_to_x(point) else {
                continue;
            };
            if x < -8.0 || x > self.chart_w + 8.0 {
                continue;
            }
            let dist = (self.pos.x - x).abs();
            if dist < best_dist {
                best_dist = dist;
                best = Some(point);
            }
        }

        if best_dist > 10.0 {
            return;
        }
        let Some(point) = best else {
            return;
        };

        let display_rate = if annualized {
            point.rate * FUNDING_RATE_ANNUALIZATION_FACTOR
        } else {
            point.rate
        };
        let rate_color = if display_rate >= 0.0 {
            self.theme.palette().success
        } else {
            self.theme.palette().danger
        };
        let lines = vec![
            TooltipLine {
                content: if annualized {
                    "Funding APR".to_string()
                } else {
                    "Funding 1H".to_string()
                },
                color: Color {
                    a: 0.70,
                    ..self.theme.palette().text
                },
            },
            TooltipLine {
                content: format_funding_rate_percent(display_rate, annualized),
                color: rate_color,
            },
            TooltipLine {
                content: format_timestamp_exact(point.time_ms),
                color: Color {
                    a: 0.62,
                    ..self.theme.palette().text
                },
            },
        ];

        let line_h: f32 = 14.0;
        let pad: f32 = 6.0;
        let card_w: f32 = 112.0;
        let card_h = lines.len() as f32 * line_h + pad * 2.0;
        let card_x = (self.pos.x + 12.0)
            .min(self.chart_w - card_w - 4.0)
            .max(4.0);
        let max_y = (panel_y + panel_h - card_h - 4.0).max(panel_y + 4.0);
        let card_y = (panel_y + 4.0).min(max_y);

        self.draw_card(
            Point::new(card_x, card_y),
            Size::new(card_w, card_h),
            &lines,
        );
    }
}
