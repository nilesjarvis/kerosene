use super::super::plot::price_axis_left;
use super::MarkerGroup;
use crate::app_time::DisplayTimeZone;
use crate::helpers::{format_decimal_with_commas, format_price, trim_decimal_zeros};
use crate::journal_views::style::{journal_hairline, tooltip_origin};
use iced::widget::canvas;
use iced::{Color, Point, Size, Theme};

impl MarkerGroup {
    pub(in crate::journal_views::trade_card::snapshot) fn draw_tooltip(
        &self,
        frame: &mut canvas::Frame,
        theme: &Theme,
        coin: &str,
        zone: DisplayTimeZone,
    ) {
        let side = if self.is_buy { "Buy" } else { "Sell" };
        let number = |value| trim_decimal_zeros(format_decimal_with_commas(value, 8));
        let (header, price, size_label) = if self.count == 1 {
            (
                format!("{side} fill"),
                format!("Price  {}", number(self.average_price)),
                "Size",
            )
        } else {
            (
                format!("{side} · {} fills", self.count),
                format!("VWAP  {}", format_price(self.average_price)),
                "Total size",
            )
        };
        let time = |ms| zone.format(ms, "%b %d, %H:%M:%S%.3f", "Unknown");
        let palette = theme.extended_palette().background;
        let mut lines = vec![
            (header, self.color(theme)),
            (price, palette.base.text),
            (
                format!("{size_label}  {} {coin}", number(self.size)),
                palette.base.text,
            ),
            (time(self.first_ms), palette.weak.text),
        ];
        if self.last_ms != self.first_ms {
            lines.push((format!("→ {}", time(self.last_ms)), palette.weak.text));
        }

        let padding = 7.0;
        let line_height = 15.0;
        let chars = lines
            .iter()
            .map(|(text, _)| text.chars().count())
            .max()
            .unwrap_or(0);
        let size = Size::new(
            (chars as f32 * 7.0 + 2.0 * padding).min(frame.width()),
            lines.len() as f32 * line_height + 2.0 * padding,
        );
        // Keep the price scale clear when the card fits inside the plot.
        let tooltip_area_width = price_axis_left(frame.size()).max(size.width);
        let origin = tooltip_origin(self.center, tooltip_area_width, frame.height(), size);
        let card = canvas::Path::rounded_rectangle(origin, size, 4.0.into());
        frame.fill(
            &card,
            Color {
                a: 1.0,
                ..palette.base.color
            },
        );
        frame.stroke(
            &card,
            canvas::Stroke::default()
                .with_color(journal_hairline(theme))
                .with_width(1.0),
        );
        for (i, (content, color)) in lines.into_iter().enumerate() {
            frame.fill_text(canvas::Text {
                content,
                position: Point::new(
                    origin.x + padding,
                    origin.y + padding + i as f32 * line_height,
                ),
                size: iced::Pixels(11.0),
                font: crate::app_fonts::monospace_font(),
                color,
                ..canvas::Text::default()
            });
        }
    }
}
