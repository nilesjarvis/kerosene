use crate::api::Candle;
use crate::message::Message;
use iced::widget::canvas;
use iced::{Point, Rectangle};

mod data;
mod hud_safety;
mod input;
mod orders;
mod view_state;
mod viewport;

fn candle_at(open_time: u64, close: f64) -> Candle {
    Candle {
        open_time,
        close_time: open_time + 59_999,
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 10.0,
    }
}

fn chart_bounds(width: f32, height: f32) -> Rectangle {
    Rectangle::new(Point::ORIGIN, iced::Size::new(width, height))
}

fn action_or_panic(
    action: Option<canvas::Action<Message>>,
    context: &str,
) -> canvas::Action<Message> {
    match action {
        Some(action) => action,
        None => panic!("{context}"),
    }
}

fn message_or_panic(message: Option<Message>, context: &str) -> Message {
    match message {
        Some(message) => message,
        None => panic!("{context}"),
    }
}
