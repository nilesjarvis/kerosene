use crate::app_state::TradingTerminal;
use crate::message::Message;
#[cfg(target_os = "linux")]
use iced::mouse;
#[cfg(target_os = "linux")]
use iced::widget::{Space, column, mouse_area, row, stack};
use iced::{Element, window};
#[cfg(target_os = "linux")]
use iced::{Fill, Length};

#[cfg(target_os = "linux")]
const RESIZE_EDGE_THICKNESS: f32 = 6.0;
#[cfg(target_os = "linux")]
const RESIZE_CORNER_SIZE: f32 = 16.0;

impl TradingTerminal {
    pub(super) fn view_with_resize_handles<'a>(
        &'a self,
        _window_id: window::Id,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        #[cfg(target_os = "linux")]
        let content = stack![content, self.view_window_resize_handles(_window_id)]
            .width(Fill)
            .height(Fill)
            .into();
        content
    }

    #[cfg(target_os = "linux")]
    fn view_window_resize_handles(&self, window_id: window::Id) -> Element<'static, Message> {
        let side_edges: Element<'static, Message> = row![
            resize_handle(
                window_id,
                window::Direction::West,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                Length::Fill,
                mouse::Interaction::ResizingHorizontally,
            ),
            Space::new().width(Fill),
            resize_handle(
                window_id,
                window::Direction::East,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                Length::Fill,
                mouse::Interaction::ResizingHorizontally,
            )
        ]
        .width(Fill)
        .height(Fill)
        .into();

        let top_bottom_edges: Element<'static, Message> = column![
            resize_handle(
                window_id,
                window::Direction::North,
                Length::Fill,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                mouse::Interaction::ResizingVertically,
            ),
            Space::new().height(Fill),
            resize_handle(
                window_id,
                window::Direction::South,
                Length::Fill,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                mouse::Interaction::ResizingVertically,
            )
        ]
        .width(Fill)
        .height(Fill)
        .into();

        let corners: Element<'static, Message> = column![
            row![
                resize_handle(
                    window_id,
                    window::Direction::NorthWest,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyDown,
                ),
                Space::new().width(Fill),
                resize_handle(
                    window_id,
                    window::Direction::NorthEast,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyUp,
                )
            ]
            .width(Fill),
            Space::new().height(Fill),
            row![
                resize_handle(
                    window_id,
                    window::Direction::SouthWest,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyUp,
                ),
                Space::new().width(Fill),
                resize_handle(
                    window_id,
                    window::Direction::SouthEast,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyDown,
                )
            ]
            .width(Fill)
        ]
        .width(Fill)
        .height(Fill)
        .into();

        stack![side_edges, top_bottom_edges, corners]
            .width(Fill)
            .height(Fill)
            .into()
    }
}

#[cfg(target_os = "linux")]
fn resize_handle(
    window_id: window::Id,
    direction: window::Direction,
    width: Length,
    height: Length,
    interaction: mouse::Interaction,
) -> Element<'static, Message> {
    mouse_area(Space::new().width(width).height(height))
        .on_press(Message::WindowDragResize(window_id, direction))
        .interaction(interaction)
        .into()
}
