use iced::Point;
use std::fmt;

mod layout;
#[cfg(test)]
mod tests;
mod tooltip;

pub(super) use self::layout::{hovered_income_bar, prepare_income_chart_layout};
pub(super) use self::tooltip::income_tooltip_layout;

// ---------------------------------------------------------------------------
// Income Projection Series
// ---------------------------------------------------------------------------

pub(super) const LEFT_PAD: f32 = 12.0;
pub(super) const RIGHT_PAD: f32 = 12.0;
pub(super) const TOP_PAD: f32 = 18.0;
pub(super) const BOTTOM_PAD: f32 = 24.0;
pub(super) const BAR_HEIGHT_RATIO: f32 = 0.42;
pub(super) const MIN_SCALE: f64 = 1e-9;
pub(super) const TOOLTIP_HEIGHT: f32 = 38.0;

#[derive(Clone, PartialEq)]
pub(super) struct IncomeBarLayout {
    pub(super) label: String,
    pub(super) value: f64,
    pub(super) center_x: f32,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) scaled: f32,
    pub(super) show_axis_label: bool,
}

impl fmt::Debug for IncomeBarLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IncomeBarLayout")
            .field("label", &"<redacted>")
            .field("value", &"<redacted>")
            .field("geometry", &"<redacted>")
            .field("scaled", &"<redacted>")
            .field("show_axis_label", &self.show_axis_label)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct IncomeChartLayout {
    pub(super) bars: Vec<IncomeBarLayout>,
    pub(super) left_pad: f32,
    pub(super) top_pad: f32,
    pub(super) bottom_pad: f32,
    pub(super) plot_width: f32,
    pub(super) plot_height: f32,
    pub(super) zero_y: f32,
    pub(super) group_width: f32,
}

impl fmt::Debug for IncomeChartLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IncomeChartLayout")
            .field("bars_count", &self.bars.len())
            .field("geometry", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct IncomeTooltipLayout {
    pub(super) origin: Point,
    pub(super) width: f32,
    pub(super) height: f32,
}
