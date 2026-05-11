mod rendering;
mod series;

use self::rendering::draw_portfolio_pnl_chart;

use crate::message::Message;

use iced::widget::canvas;
use iced::{Rectangle, Renderer, Theme};

// ---------------------------------------------------------------------------
// Portfolio PnL Chart
// ---------------------------------------------------------------------------

pub(crate) struct PortfolioPnlChart {
    pub(crate) points: Vec<(u64, f64)>,
    pub(crate) value_mode: PnlValueDisplayMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PnlValueDisplayMode {
    Usd,
    Percent,
}

impl canvas::Program<Message> for PortfolioPnlChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        draw_portfolio_pnl_chart(
            &self.points,
            self.value_mode,
            renderer,
            theme,
            bounds,
            cursor,
        )
    }
}
