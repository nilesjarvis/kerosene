use crate::helpers::ease_out_cubic;
use crate::message::Message;

use super::{MenuAlignment, MenuKind};
use iced::advanced::renderer::Renderer as _;
use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, overlay, renderer};
use iced::{Element, Event, Point, Rectangle, Size, Theme};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Anchored Menu Overlay
// ---------------------------------------------------------------------------

const MENU_VERTICAL_GAP: f32 = 6.0;
const MENU_HORIZONTAL_INSET: f32 = 12.0;
const MENU_VIEWPORT_MARGIN: f32 = 4.0;

/// Duration of the open reveal animation.
const MENU_OPEN_DURATION: Duration = Duration::from_millis(140);
/// Vertical distance the menu slides into place while opening.
const MENU_OPEN_SLIDE: f32 = 8.0;

// ---------------------------------------------------------------------------
// Animation State
// ---------------------------------------------------------------------------

/// Persistent open-animation state stored in the anchored-menu widget tree.
///
/// The overlay is only present while a menu is open, so we drive a short
/// reveal animation from a fresh `progress` of `0.0` each time it mounts.
#[derive(Debug, Default)]
pub(in crate::account_views::summary::menus) struct MenuAnimation {
    progress: f32,
    last_frame: Option<Instant>,
    mounted: Option<MenuKind>,
}

impl MenuAnimation {
    /// Arms the reveal so that it plays from the start the next time a menu
    /// mounts. Called while no menu is showing.
    pub(in crate::account_views::summary::menus) fn reset(&mut self) {
        self.mounted = None;
        self.progress = 0.0;
        self.last_frame = None;
    }

    /// Restarts the reveal whenever a different menu mounts so each distinct
    /// menu animates open, even when switching directly between them.
    fn ensure_mounted(&mut self, kind: MenuKind) {
        if self.mounted != Some(kind) {
            self.mounted = Some(kind);
            self.progress = 0.0;
            self.last_frame = None;
        }
    }

    /// Advances the animation based on elapsed wall-clock time, returning
    /// `true` while the reveal is still in progress.
    fn advance(&mut self, now: Instant) -> bool {
        let delta = self
            .last_frame
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        self.last_frame = Some(now);

        if self.progress < 1.0 {
            let step = delta.as_secs_f32() / MENU_OPEN_DURATION.as_secs_f32().max(f32::EPSILON);
            self.progress = (self.progress + step).min(1.0);
        }

        self.progress < 1.0
    }

    fn eased(&self) -> f32 {
        ease_out_cubic(self.progress)
    }
}

pub(super) struct AnchoredMenuOverlay<'a, 'b> {
    pub(super) content: &'b mut Element<'a, Message>,
    pub(super) tree: &'b mut tree::Tree,
    pub(super) animation: &'b mut MenuAnimation,
    pub(super) kind: MenuKind,
    pub(super) anchor: Rectangle,
    pub(super) alignment: MenuAlignment,
    pub(super) viewport: Rectangle,
}

impl AnchoredMenuOverlay<'_, '_> {
    /// The top-left corner where the menu rests once fully open.
    fn resting_position(&self, menu_size: Size) -> Point {
        let x = match self.alignment {
            MenuAlignment::Start => self.anchor.x + MENU_HORIZONTAL_INSET,
            MenuAlignment::End => {
                self.anchor.x + self.anchor.width - menu_size.width - MENU_HORIZONTAL_INSET
            }
        };
        let x = clamp_to_viewport(x, menu_size.width, self.viewport.x, self.viewport.width);

        // Open directly below the anchoring bar so the dropdown never overlaps
        // the button that triggered it.
        let preferred_y = self.anchor.y + self.anchor.height + MENU_VERTICAL_GAP;
        let y = if preferred_y + menu_size.height + MENU_VIEWPORT_MARGIN
            <= self.viewport.y + self.viewport.height
        {
            preferred_y
        } else {
            self.anchor.y - menu_size.height - MENU_VERTICAL_GAP
        };
        let y = clamp_to_viewport(y, menu_size.height, self.viewport.y, self.viewport.height);

        Point::new(x, y)
    }
}

impl overlay::Overlay<Message, Theme, iced::Renderer> for AnchoredMenuOverlay<'_, '_> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);
        self.viewport = viewport;

        self.animation.ensure_mounted(self.kind);

        let mut node = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let menu_size = node.size();

        let position = self.resting_position(menu_size);

        // Slide the menu down into place while opening. Layout reflects the
        // animated offset so pointer hit-testing matches what is drawn.
        let slide = (1.0 - self.animation.eased()) * MENU_OPEN_SLIDE;
        node.move_to_mut(Point::new(position.x, position.y - slide));
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
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event
            && self.animation.advance(*now)
        {
            shell.request_redraw();
        }

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
        let bounds = layout.bounds();

        // Reveal the menu by clipping it to a height that grows from the top
        // edge while opening, producing a subtle unfold.
        let revealed_height = (bounds.height * self.animation.eased()).max(0.0);
        let clip = Rectangle {
            height: revealed_height,
            ..bounds
        };

        renderer.with_layer(clip, |renderer| {
            self.content.as_widget().draw(
                self.tree,
                renderer,
                theme,
                style,
                layout,
                cursor,
                &self.viewport,
            );
        });
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
