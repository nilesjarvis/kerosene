use crate::alfred_state::AlfredCommand;

pub(super) trait AlfredCommandAvailability {
    fn disabled_if(self, condition: bool, reason: &'static str) -> Self;
}

impl AlfredCommandAvailability for AlfredCommand {
    fn disabled_if(self, condition: bool, reason: &'static str) -> Self {
        if condition {
            self.disabled(reason)
        } else {
            self
        }
    }
}

pub(super) fn open_tag(open: bool, closed_tag: &'static str) -> &'static str {
    if open { "Open" } else { closed_tag }
}

pub(super) fn income_tag(open: bool, can_add_income: bool) -> &'static str {
    if open {
        "Open"
    } else if can_add_income {
        "Pane"
    } else {
        "Requires PM"
    }
}
