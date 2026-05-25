use super::super::super::CandleLayerContext;
use crate::chart::model::{
    CandlestickChart, FUNDING_MODE_BUTTON_HEIGHT, FUNDING_MODE_BUTTON_WIDTH, FUNDING_MODE_BUTTON_X,
    FUNDING_MODE_BUTTON_Y_OFFSET,
};

use iced::alignment;
use iced::widget::canvas;
use iced::{Color, Point, Size};

// ---------------------------------------------------------------------------
// Funding Mode Button
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn draw_funding_mode_button<IdxToCx, PriceToY>(
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
            font: crate::app_fonts::monospace_font(),
            ..canvas::Text::default()
        });
    }
}
