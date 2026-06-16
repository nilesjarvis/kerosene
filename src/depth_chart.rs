use iced::mouse;
use iced::widget::canvas::{self, Action};
use iced::{Event, Point, Rectangle, Renderer, Theme};

use crate::market_state::OrderBookId;
use crate::message::Message;

mod geometry;
mod interaction;
mod rendering;

#[derive(Default)]
pub struct DepthChartState {
    pub hover_pos: Option<Point>,
}

/// Cumulative depth chart of an order book: bid depth stepping up to the left
/// of the mid price, ask depth to the right, both filled to the baseline.
/// Holds owned copies of the aggregated levels because the aggregation cache
/// hands out `RefCell` guards that cannot outlive the view call.
pub struct DepthChart {
    pub id: OrderBookId,
    /// `(price, size, cumulative size)` per level, best-first (descending price).
    pub bids: Vec<(f64, f64, f64)>,
    /// `(price, size, cumulative size)` per level, best-first (ascending price).
    pub asks: Vec<(f64, f64, f64)>,
    pub mid: Option<f64>,
    pub tick: f64,
    pub decimals: usize,
    /// Outcome books trade whole contracts, so cumulative labels should not
    /// display fractional base-unit sizes.
    pub whole_contracts: bool,
    /// Tick-bucket prices holding the user's resting orders, for the
    /// baseline markers.
    pub user_bid_prices: Vec<f64>,
    pub user_ask_prices: Vec<f64>,
}

impl canvas::Program<Message> for DepthChart {
    type State = DepthChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        self.update_interaction(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        self.draw_chart(state, renderer, theme, bounds)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        self.mouse_interaction_for(bounds, cursor)
    }
}
