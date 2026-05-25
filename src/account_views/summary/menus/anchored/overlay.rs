use crate::message::Message;

use super::MenuAlignment;
use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, overlay, renderer};
use iced::{Element, Event, Point, Rectangle, Size, Theme};

// ---------------------------------------------------------------------------
// Anchored Menu Overlay
// ---------------------------------------------------------------------------

const MENU_VERTICAL_OFFSET: f32 = 34.0;
const MENU_HORIZONTAL_INSET: f32 = 12.0;
const MENU_VIEWPORT_MARGIN: f32 = 4.0;

pub(super) struct AnchoredMenuOverlay<'a, 'b> {
    pub(super) content: &'b mut Element<'a, Message>,
    pub(super) tree: &'b mut tree::Tree,
    pub(super) anchor: Rectangle,
    pub(super) alignment: MenuAlignment,
    pub(super) viewport: Rectangle,
}

impl overlay::Overlay<Message, Theme, iced::Renderer> for AnchoredMenuOverlay<'_, '_> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);
        self.viewport = viewport;

        let mut node = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let menu_size = node.size();

        let x = match self.alignment {
            MenuAlignment::Start => self.anchor.x + MENU_HORIZONTAL_INSET,
            MenuAlignment::End => {
                self.anchor.x + self.anchor.width - menu_size.width - MENU_HORIZONTAL_INSET
            }
        };
        let x = clamp_to_viewport(x, menu_size.width, viewport.x, viewport.width);

        let preferred_y = self.anchor.y + MENU_VERTICAL_OFFSET;
        let y = if preferred_y + menu_size.height + MENU_VIEWPORT_MARGIN
            <= viewport.y + viewport.height
        {
            preferred_y
        } else {
            self.anchor.y - menu_size.height - MENU_VIEWPORT_MARGIN
        };
        let y = clamp_to_viewport(y, menu_size.height, viewport.y, viewport.height);

        node.move_to_mut(Point::new(x, y));
        node
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor
                .position()
                .is_some_and(|point| !layout.bounds().contains(point))
        {
            shell.publish(Message::CloseAllMenus);
            shell.capture_event();
            return;
        }

        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &self.viewport,
        );
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &self.viewport,
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if !cursor.is_over(layout.bounds()) {
            return mouse::Interaction::None;
        }

        self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &self.viewport,
            renderer,
        )
    }

    fn index(&self) -> f32 {
        10.0
    }
}

pub(super) fn clamp_to_viewport(
    value: f32,
    size: f32,
    viewport_start: f32,
    viewport_size: f32,
) -> f32 {
    let min = viewport_start + MENU_VIEWPORT_MARGIN;
    let max = viewport_start + viewport_size - size - MENU_VIEWPORT_MARGIN;

    if max < min {
        min
    } else {
        value.clamp(min, max)
    }
}
