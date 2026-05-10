use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::opaque;
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

const MENU_VERTICAL_OFFSET: f32 = 34.0;
const MENU_HORIZONTAL_INSET: f32 = 12.0;
const MENU_VIEWPORT_MARGIN: f32 = 4.0;

// ---------------------------------------------------------------------------
// Account Summary Menus
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_account_summary_with_menus<'a>(
        &'a self,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let can_add_income = self
            .account_data
            .as_ref()
            .is_some_and(AccountData::is_portfolio_margin);

        let menu = if self.account_picker_open {
            Some(AnchoredMenuLayer {
                alignment: MenuAlignment::Start,
                content: opaque(self.view_account_picker_dropdown()),
            })
        } else if self.add_widget_menu_open {
            Some(AnchoredMenuLayer {
                alignment: MenuAlignment::End,
                content: opaque(self.view_add_widget_menu_card(&theme, can_add_income)),
            })
        } else {
            None
        };

        AnchoredAccountMenu::new(content, menu).into()
    }
}

// ---------------------------------------------------------------------------
// Anchored Overlay Widget
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum MenuAlignment {
    Start,
    End,
}

struct AnchoredMenuLayer<'a> {
    alignment: MenuAlignment,
    content: Element<'a, Message>,
}

struct AnchoredAccountMenu<'a> {
    content: Element<'a, Message>,
    menu: Option<AnchoredMenuLayer<'a>>,
}

impl<'a> AnchoredAccountMenu<'a> {
    fn new(content: Element<'a, Message>, menu: Option<AnchoredMenuLayer<'a>>) -> Self {
        Self { content, menu }
    }
}

impl Widget<Message, Theme, iced::Renderer> for AnchoredAccountMenu<'_> {
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
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let (content_trees, menu_trees) = tree.children.split_at_mut(1);
        let content_overlay = self.content.as_widget_mut().overlay(
            &mut content_trees[0],
            layout,
            renderer,
            viewport,
            translation,
        );

        let menu_overlay = self.menu.as_mut().map(|menu| {
            overlay::Element::new(Box::new(AnchoredMenuOverlay {
                content: &mut menu.content,
                tree: &mut menu_trees[0],
                anchor: layout.bounds() + translation,
                alignment: menu.alignment,
                viewport: *viewport,
            }))
        });

        match (content_overlay, menu_overlay) {
            (None, None) => None,
            (Some(overlay), None) | (None, Some(overlay)) => Some(overlay),
            (Some(content), Some(menu)) => {
                Some(overlay::Group::with_children(vec![content, menu]).overlay())
            }
        }
    }
}

impl<'a> From<AnchoredAccountMenu<'a>> for Element<'a, Message> {
    fn from(menu: AnchoredAccountMenu<'a>) -> Self {
        Element::new(menu)
    }
}

struct AnchoredMenuOverlay<'a, 'b> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut tree::Tree,
    anchor: Rectangle,
    alignment: MenuAlignment,
    viewport: Rectangle,
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

fn clamp_to_viewport(value: f32, size: f32, viewport_start: f32, viewport_size: f32) -> f32 {
    let min = viewport_start + MENU_VIEWPORT_MARGIN;
    let max = viewport_start + viewport_size - size - MENU_VIEWPORT_MARGIN;

    if max < min {
        min
    } else {
        value.clamp(min, max)
    }
}
