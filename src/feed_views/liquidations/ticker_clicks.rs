use crate::message::Message;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse};

pub(super) fn with_ticker_clicks(content: Element<'_, Message>) -> Element<'_, Message> {
    Element::new(TickerClicks { content })
}

/// Track modifiers at the feed level so new rows also honor a held Ctrl key.
struct TickerClicks<'a> {
    content: Element<'a, Message>,
}

impl Widget<Message, Theme, Renderer> for TickerClicks<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<keyboard::Modifiers>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(keyboard::Modifiers::NONE)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.content.as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let modifiers = tree.state.downcast_mut::<keyboard::Modifiers>();
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(current))
            | Event::Keyboard(keyboard::Event::KeyPressed {
                modifiers: current, ..
            })
            | Event::Keyboard(keyboard::Event::KeyReleased {
                modifiers: current, ..
            }) => {
                *modifiers = *current;
            }
            Event::Window(iced::window::Event::Unfocused) => {
                *modifiers = keyboard::Modifiers::NONE;
            }
            _ => {}
        }
        let open_window = modifiers.control();
        let mut messages = Vec::new();
        let mut child_shell = Shell::new(&mut messages);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut child_shell,
            viewport,
        );
        shell.merge(child_shell, |message| match message {
            Message::SymbolSelected(symbol) if open_window => {
                Message::OpenDetachedChartForSymbol(symbol)
            }
            message => message,
        });
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
