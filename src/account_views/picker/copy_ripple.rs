use crate::helpers::ease_out_cubic;
use crate::message::Message;

use iced::advanced::renderer::Renderer as _;
use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{
    Background, Color, Element, Event, Length, Point, Rectangle, Size, Theme, Vector, border,
};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Copy Ripple
// ---------------------------------------------------------------------------
//
// Wraps the account address button so that clicking it plays a Material-style
// ripple originating from the exact click position, signalling that the
// address was copied to the clipboard. The wrapper only observes presses; it
// never captures them, so the inner button still fires its copy action.

/// Total lifetime of a single ripple animation.
const RIPPLE_DURATION: Duration = Duration::from_millis(520);
/// Peak opacity of the ripple fill at the start of the animation.
const RIPPLE_PEAK_ALPHA: f32 = 0.32;

/// Transient state for the ripple currently playing, if any.
#[derive(Debug, Clone, Copy)]
struct Ripple {
    /// Click position relative to the widget's top-left corner.
    origin: Point,
    /// Set from iced's redraw clock on the first animation frame.
    started_at: Option<Instant>,
    progress: f32,
}

#[derive(Debug, Default)]
pub(super) struct RippleState {
    ripple: Option<Ripple>,
}

impl RippleState {
    /// Advances the active ripple against the redraw clock, clearing it once it
    /// has fully faded. Returns `true` while the ripple is still animating.
    fn advance(&mut self, now: Instant) -> bool {
        let Some(ripple) = self.ripple.as_mut() else {
            return false;
        };

        let started_at = *ripple.started_at.get_or_insert(now);
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed >= RIPPLE_DURATION {
            self.ripple = None;
            false
        } else {
            ripple.progress = ripple_progress(elapsed);
            true
        }
    }

    fn start(&mut self, origin: Point) {
        self.ripple = Some(Ripple {
            origin,
            started_at: None,
            progress: 0.0,
        });
    }
}

pub(super) struct CopyRipple<'a> {
    content: Element<'a, Message>,
    tint: Color,
}

impl<'a> CopyRipple<'a> {
    pub(super) fn new(content: impl Into<Element<'a, Message>>, tint: Color) -> Self {
        Self {
            content: content.into(),
            tint,
        }
    }
}

impl Widget<Message, Theme, iced::Renderer> for CopyRipple<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<RippleState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(RippleState::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(&[self.content.as_widget()]);
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
        let state = tree.state.downcast_mut::<RippleState>();
        let bounds = layout.bounds();

        match event {
            Event::Window(iced::window::Event::RedrawRequested(now)) if state.advance(*now) => {
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_over(bounds) {
                    state.start(Point::new(position.x - bounds.x, position.y - bounds.y));
                    shell.request_redraw();
                }
            }
            _ => {}
        }

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

        let state = tree.state.downcast_ref::<RippleState>();
        let Some(ripple) = state.ripple else {
            return;
        };

        draw_ripple(
            renderer,
            layout.bounds(),
            ripple.origin,
            ripple.progress,
            self.tint,
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
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a> From<CopyRipple<'a>> for Element<'a, Message> {
    fn from(ripple: CopyRipple<'a>) -> Self {
        Element::new(ripple)
    }
}

/// Draws the expanding, fading ripple disc clipped to the address segment.
fn draw_ripple(
    renderer: &mut iced::Renderer,
    bounds: Rectangle,
    origin: Point,
    progress: f32,
    tint: Color,
) {
    let radius = ripple_radius(bounds.size(), origin, progress);
    if radius <= 0.0 {
        return;
    }

    let alpha = ripple_alpha(progress);
    if alpha <= 0.0 {
        return;
    }

    let center = Point::new(bounds.x + origin.x, bounds.y + origin.y);
    let disc = Rectangle {
        x: center.x - radius,
        y: center.y - radius,
        width: radius * 2.0,
        height: radius * 2.0,
    };

    // Clip the ripple to the segment so it never spills past the rounded edges.
    renderer.with_layer(bounds, |renderer| {
        renderer.fill_quad(
            renderer::Quad {
                bounds: disc,
                border: border::rounded(radius),
                ..Default::default()
            },
            Background::Color(Color { a: alpha, ..tint }),
        );
    });
}

/// Expanding radius eased so the ripple grows quickly then settles.
fn ripple_radius(size: Size, origin: Point, progress: f32) -> f32 {
    let max_radius = max_ripple_radius(size, origin);
    max_radius * ease_out_cubic(progress)
}

fn ripple_progress(elapsed: Duration) -> f32 {
    (elapsed.as_secs_f32() / RIPPLE_DURATION.as_secs_f32()).clamp(0.0, 1.0)
}

/// Distance from the click origin to the farthest corner of the segment.
fn max_ripple_radius(size: Size, origin: Point) -> f32 {
    let dx = origin.x.max(size.width - origin.x);
    let dy = origin.y.max(size.height - origin.y);
    (dx * dx + dy * dy).sqrt()
}

/// Opacity envelope: hold near the peak early, then fade out smoothly.
fn ripple_alpha(progress: f32) -> f32 {
    let fade = 1.0 - progress.clamp(0.0, 1.0);
    RIPPLE_PEAK_ALPHA * fade * fade
}

#[cfg(test)]
mod tests;
