use crate::message::Message;

use iced::widget::{Space, canvas};
use iced::{Element, Point, Rectangle, Renderer, Theme};

const USER_ORDER_MARKER_WIDTH: f32 = 10.0;
const USER_ORDER_MARKER_HEIGHT: f32 = 10.0;
const USER_ORDER_MARKER_RADIUS: f32 = 3.1;

pub fn user_order_price_marker(user_order_side: Option<bool>) -> Element<'static, Message> {
    let Some(is_bid) = user_order_side else {
        return Space::new()
            .width(USER_ORDER_MARKER_WIDTH)
            .height(USER_ORDER_MARKER_HEIGHT)
            .into();
    };

    canvas(UserOrderPriceMarker { is_bid })
        .width(USER_ORDER_MARKER_WIDTH)
        .height(USER_ORDER_MARKER_HEIGHT)
        .into()
}

struct UserOrderPriceMarker {
    is_bid: bool,
}

impl canvas::Program<Message> for UserOrderPriceMarker {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let radius = USER_ORDER_MARKER_RADIUS.min((bounds.width.min(bounds.height) / 2.0).max(0.0));
        if radius <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let color = if self.is_bid {
            theme.palette().success
        } else {
            theme.palette().danger
        };
        let path =
            canvas::Path::circle(Point::new(bounds.width / 2.0, bounds.height / 2.0), radius);
        frame.fill(&path, color);
        vec![frame.into_geometry()]
    }
}
