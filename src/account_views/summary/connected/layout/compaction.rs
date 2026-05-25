use super::super::super::CONNECTED_SUMMARY_ACTION_BREAKPOINT;

// ---------------------------------------------------------------------------
// Connected Summary Compaction
// ---------------------------------------------------------------------------

const HIDE_DISPLAY_DENOMINATION_SELECTOR_WIDTH: f32 = CONNECTED_SUMMARY_ACTION_BREAKPOINT;
const HIDE_SOUND_SELECTOR_WIDTH: f32 = 1_020.0;
const HIDE_NOTIFICATION_SELECTOR_WIDTH: f32 = 940.0;
const HIDE_MARGIN_RATIO_WIDTH: f32 = 840.0;
const HIDE_MARGIN_USED_WIDTH: f32 = 720.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ConnectedSummaryCompaction {
    hidden_priority_count: u8,
}

impl ConnectedSummaryCompaction {
    pub(super) const fn for_width(width: f32) -> Self {
        let hidden_priority_count = if width < HIDE_MARGIN_USED_WIDTH {
            5
        } else if width < HIDE_MARGIN_RATIO_WIDTH {
            4
        } else if width < HIDE_NOTIFICATION_SELECTOR_WIDTH {
            3
        } else if width < HIDE_SOUND_SELECTOR_WIDTH {
            2
        } else if width < HIDE_DISPLAY_DENOMINATION_SELECTOR_WIDTH {
            1
        } else {
            0
        };

        Self {
            hidden_priority_count,
        }
    }

    pub(super) const fn hide_display_denomination(self) -> bool {
        self.hidden_priority_count >= 1
    }

    pub(super) const fn hide_sound(self) -> bool {
        self.hidden_priority_count >= 2
    }

    pub(super) const fn hide_notifications(self) -> bool {
        self.hidden_priority_count >= 3
    }

    pub(super) const fn hide_margin_ratio(self) -> bool {
        self.hidden_priority_count >= 4
    }

    pub(super) const fn hide_margin_used(self) -> bool {
        self.hidden_priority_count >= 5
    }
}
