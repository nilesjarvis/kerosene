use iced::Rectangle;
use iced::advanced::widget::{Id, Operation, operation::Outcome};

// ---------------------------------------------------------------------------
// Screenshot Bounds
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(super) struct FindWidgetBounds {
    target: Id,
    bounds: Option<Rectangle>,
}

impl FindWidgetBounds {
    pub(super) fn new(target: Id) -> Self {
        Self {
            target,
            bounds: None,
        }
    }
}

impl Operation<Option<Rectangle>> for FindWidgetBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Rectangle>>)) {
        if self.bounds.is_none() {
            operate(self);
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            self.bounds = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<Option<Rectangle>> {
        Outcome::Some(self.bounds)
    }
}
