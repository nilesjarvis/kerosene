use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::pane_grid;
use iced::{Size, Task};

mod layout;
pub(in crate::pane_interaction_update) use layout::clamp_split_ratio;
use layout::{split_node, subtree_contains_order_entry, subtree_min_length};

const MAIN_STATUS_BAR_RESERVED_HEIGHT: f32 = 28.0;

// ---------------------------------------------------------------------------
// Pane Minimum Sizing
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn clamp_order_entry_resize_ratio(
        &self,
        split: pane_grid::Split,
        ratio: f32,
    ) -> f32 {
        let base_min_size = self.pane_grid_min_size();
        let size = self.main_pane_grid_size();
        let pane_border_thickness = self.pane_border_thickness;
        let split_regions =
            self.panes
                .layout()
                .split_regions(pane_border_thickness, base_min_size, size);
        let Some((axis, region, _current_ratio)) = split_regions.get(&split).copied() else {
            return ratio;
        };
        let Some((_, a, b)) = split_node(self.panes.layout(), split) else {
            return ratio;
        };

        let order_entry_in_a = subtree_contains_order_entry(a, &self.panes);
        let order_entry_in_b = subtree_contains_order_entry(b, &self.panes);
        if !order_entry_in_a && !order_entry_in_b {
            return ratio;
        }

        let min_a = subtree_min_length(a, axis, &self.panes, base_min_size, pane_border_thickness);
        let min_b = subtree_min_length(b, axis, &self.panes, base_min_size, pane_border_thickness);
        let axis_length = match axis {
            pane_grid::Axis::Horizontal => region.height,
            pane_grid::Axis::Vertical => region.width,
        };

        clamp_split_ratio(
            ratio,
            axis_length,
            min_a,
            min_b,
            order_entry_in_a,
            order_entry_in_b,
            pane_border_thickness,
        )
    }

    fn main_pane_grid_size(&self) -> Size {
        let size = self.main_window_size.unwrap_or(Size::new(1600.0, 960.0));
        let exterior_padding = self.outer_widget_border_padding() * 2.0;
        Size::new(
            (size.width - exterior_padding).max(1.0),
            (size.height - self.main_chrome_height()).max(1.0),
        )
    }

    pub(crate) fn main_window_min_size(&self) -> Size {
        let base_min_size = self.pane_grid_min_size();
        let layout = self.panes.layout();

        Size::new(
            subtree_min_length(
                layout,
                pane_grid::Axis::Vertical,
                &self.panes,
                base_min_size,
                self.pane_border_thickness,
            ) + self.outer_widget_border_padding() * 2.0,
            subtree_min_length(
                layout,
                pane_grid::Axis::Horizontal,
                &self.panes,
                base_min_size,
                self.pane_border_thickness,
            ) + self.main_chrome_height(),
        )
    }

    pub(crate) fn sync_main_window_min_size(&self) -> Task<Message> {
        self.main_window_id
            .map(|id| iced::window::set_min_size(id, Some(self.main_window_min_size())))
            .unwrap_or_else(Task::none)
    }

    fn main_chrome_height(&self) -> f32 {
        let ticker_height = self.ticker_tape_bar_height();
        let ticker_gap = if ticker_height > 0.0 {
            self.pane_border_thickness
        } else {
            0.0
        };

        MAIN_STATUS_BAR_RESERVED_HEIGHT
            + self.account_summary_bar_height()
            + ticker_height
            + ticker_gap
            + self.pane_border_thickness * 2.0
    }

    pub(crate) fn outer_widget_border_padding(&self) -> f32 {
        if self.outer_widget_border_enabled {
            self.pane_border_thickness
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::TradingTerminal;
    use crate::config::KeroseneConfig;

    fn terminal_with_outer_widget_border(enabled: bool) -> TradingTerminal {
        let config = KeroseneConfig {
            main_window_width: Some(1600.0),
            main_window_height: Some(960.0),
            outer_widget_border_enabled: enabled,
            ..KeroseneConfig::default()
        };
        let (terminal, _task) = TradingTerminal::boot_from_config(config);
        terminal
    }

    #[test]
    fn outer_widget_border_padding_tracks_toggle() {
        let disabled = terminal_with_outer_widget_border(false);
        let enabled = terminal_with_outer_widget_border(true);

        assert_eq!(disabled.outer_widget_border_padding(), 0.0);
        assert_eq!(
            enabled.outer_widget_border_padding(),
            enabled.pane_border_thickness
        );
    }

    #[test]
    fn outer_widget_border_updates_main_window_width_sizing() {
        let disabled = terminal_with_outer_widget_border(false);
        let enabled = terminal_with_outer_widget_border(true);
        let exterior_padding = enabled.pane_border_thickness * 2.0;

        assert_eq!(
            enabled.main_pane_grid_size().width,
            disabled.main_pane_grid_size().width - exterior_padding
        );
        assert_eq!(enabled.main_chrome_height(), disabled.main_chrome_height());
        assert_eq!(
            enabled.main_window_min_size().width,
            disabled.main_window_min_size().width + exterior_padding
        );
        assert_eq!(
            enabled.main_window_min_size().height,
            disabled.main_window_min_size().height
        );
    }
}
