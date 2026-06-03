#[cfg(test)]
mod tests;

mod overlay;

use self::overlay::{AnchoredMenuOverlay, MenuAnimation};
use crate::message::Message;

use iced::advanced::widget::tree;
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay as iced_overlay, renderer,
};
use iced::{Element, Event, Length, Rectangle, Size, Theme, Vector};

#[cfg(test)]
use overlay::{clamp_to_viewport, ease_out_cubic};

// ---------------------------------------------------------------------------
// Anchored Overlay Widget
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) enum MenuAlignment {
    Start,
    End,
}

/// Identifies which summary menu is being shown so the reveal animation can
/// replay when switching directly between different menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuKind {
    AccountPicker,
    LayoutSwitcher,
    AddWidget,
}

pub(super) struct AnchoredMenuLayer<'a> {
    pub(super) kind: MenuKind,
    pub(super) alignment: MenuAlignment,
    pub(super) content: Element<'a, Message>,
}

pub(super) struct AnchoredAccountMenu<'a> {
    content: Element<'a, Message>,
    menu: Option<AnchoredMenuLayer<'a>>,
}

impl<'a> AnchoredAccountMenu<'a> {
    pub(super) fn new(content: Element<'a, Message>, menu: Option<AnchoredMenuLayer<'a>>) -> Self {
        Self { content, menu }
    }
}

impl Widget<Message, Theme, iced::Renderer> for AnchoredAccountMenu<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<MenuAnimation>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(MenuAnimation::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        let mut children = vec![tree::Tree::new(&self.content)];
        if let Some(menu) = &self.menu {
            children.push(tree::Tree::new(&menu.content));
        }
        children
    }

    fn diff(&self, tree: &mut tree::Tree) {
        match &self.menu {
            Some(menu) => tree.diff_children(&[self.content.as_widget(), menu.content.as_widget()]),
            None => tree.diff_children(&[self.content.as_widget()]),
        }
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
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
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
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
        tree: &mut tree::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced_overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let animation = tree.state.downcast_mut::<MenuAnimation>();
        let (content_trees, menu_trees) = tree.children.split_at_mut(1);
        let content_overlay = self.content.as_widget_mut().overlay(
            &mut content_trees[0],
            layout,
            renderer,
            viewport,
            translation,
        );

        let menu_overlay = match self.menu.as_mut() {
            Some(menu) => Some(iced_overlay::Element::new(Box::new(AnchoredMenuOverlay {
                content: &mut menu.content,
                tree: &mut menu_trees[0],
                animation,
                kind: menu.kind,
                anchor: layout.bounds() + translation,
                alignment: menu.alignment,
                viewport: *viewport,
            }))),
            None => {
                // No menu is showing, so arm the reveal for the next open.
                animation.reset();
                None
            }
        };

        match (content_overlay, menu_overlay) {
            (None, None) => None,
            (Some(overlay), None) | (None, Some(overlay)) => Some(overlay),
            (Some(content), Some(menu)) => {
                Some(iced_overlay::Group::with_children(vec![content, menu]).overlay())
            }
        }
    }
}

impl<'a> From<AnchoredAccountMenu<'a>> for Element<'a, Message> {
    fn from(menu: AnchoredAccountMenu<'a>) -> Self {
        Element::new(menu)
    }
}
