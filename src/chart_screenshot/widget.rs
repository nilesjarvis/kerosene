use crate::chart::{CandlestickChart, ChartState};
use crate::message::Message;

use iced::advanced::widget::{Id, Operation, operation::Outcome, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::widget::canvas::Canvas;
use iced::{Element, Event, Fill, Length, Rectangle, Renderer, Size, Theme, mouse};
use std::any::Any;
use std::sync::Arc;

/// A canvas with a read-only capture operation. Delegate its tree tag, state,
/// layout and interaction to iced so screenshot support cannot reset the view.
pub(crate) struct ScreenshotCanvas<'a> {
    canvas: Canvas<&'a CandlestickChart, Message>,
    id: Id,
}

impl<'a> ScreenshotCanvas<'a> {
    pub(crate) fn new(chart: &'a CandlestickChart, id: Id) -> Self {
        Self {
            canvas: Canvas::new(chart).width(Fill).height(Fill),
            id,
        }
    }
}

impl Widget<Message, Theme, Renderer> for ScreenshotCanvas<'_> {
    fn tag(&self) -> tree::Tag {
        self.canvas.tag()
    }

    fn state(&self) -> tree::State {
        self.canvas.state()
    }

    fn size(&self) -> Size<Length> {
        self.canvas.size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.canvas.layout(tree, renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.canvas.update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.canvas
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.canvas
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.custom(
            Some(&self.id),
            layout.bounds(),
            tree.state.downcast_mut::<ChartState>(),
        );
    }
}

impl<'a> From<ScreenshotCanvas<'a>> for Element<'a, Message> {
    fn from(canvas: ScreenshotCanvas<'a>) -> Self {
        Element::new(canvas)
    }
}

#[derive(Clone)]
pub(super) struct ChartCanvasSnapshot {
    pub(super) size: Size,
    pub(super) state: Arc<ChartState>,
}

pub(super) struct CaptureChartCanvas {
    target: Id,
    snapshot: Option<ChartCanvasSnapshot>,
}

impl CaptureChartCanvas {
    pub(super) fn new(target: Id) -> Self {
        Self {
            target,
            snapshot: None,
        }
    }
}

impl Operation<Option<ChartCanvasSnapshot>> for CaptureChartCanvas {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn Operation<Option<ChartCanvasSnapshot>>),
    ) {
        if self.snapshot.is_none() {
            operate(self);
        }
    }

    fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Any) {
        if id == Some(&self.target)
            && self.snapshot.is_none()
            && let Some(state) = state.downcast_ref::<ChartState>()
        {
            self.snapshot = Some(ChartCanvasSnapshot {
                size: bounds.size(),
                state: Arc::new(state.snapshot_for_export()),
            });
        }
    }

    fn finish(&self) -> Outcome<Option<ChartCanvasSnapshot>> {
        Outcome::Some(self.snapshot.clone())
    }
}

#[cfg(test)]
mod tests;
