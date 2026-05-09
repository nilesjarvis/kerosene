mod conversions;

use self::conversions::{pane_kind_from_config, pane_kind_to_config};
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::config::{AxisConfig, PaneKindConfig, PaneLayoutConfig};
use crate::pane_state::PaneKind;
use crate::spaghetti_state::SpaghettiChartId;
use iced::widget::pane_grid;

// ---------------------------------------------------------------------------
// Pane Layout Snapshots
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn pane_layout_to_configuration(
        layout: &PaneLayoutConfig,
    ) -> pane_grid::Configuration<PaneKind> {
        match layout {
            PaneLayoutConfig::Leaf(kind) => {
                pane_grid::Configuration::Pane(pane_kind_from_config(kind))
            }
            PaneLayoutConfig::Split { axis, ratio, a, b } => pane_grid::Configuration::Split {
                axis: match axis {
                    AxisConfig::Horizontal => pane_grid::Axis::Horizontal,
                    AxisConfig::Vertical => pane_grid::Axis::Vertical,
                },
                ratio: *ratio,
                a: Box::new(Self::pane_layout_to_configuration(a)),
                b: Box::new(Self::pane_layout_to_configuration(b)),
            },
        }
    }

    pub(crate) fn collect_layout_widget_ids(
        layout: &PaneLayoutConfig,
        chart_ids: &mut std::collections::BTreeSet<ChartId>,
        spaghetti_ids: &mut std::collections::BTreeSet<SpaghettiChartId>,
    ) {
        match layout {
            PaneLayoutConfig::Leaf(PaneKindConfig::Chart { chart_id }) => {
                chart_ids.insert(*chart_id);
            }
            PaneLayoutConfig::Leaf(PaneKindConfig::SpaghettiChart { spaghetti_id }) => {
                spaghetti_ids.insert(*spaghetti_id);
            }
            PaneLayoutConfig::Leaf(_) => {}
            PaneLayoutConfig::Split { a, b, .. } => {
                Self::collect_layout_widget_ids(a, chart_ids, spaghetti_ids);
                Self::collect_layout_widget_ids(b, chart_ids, spaghetti_ids);
            }
        }
    }

    /// Serialize the full pane tree (layout + widget placement).
    pub(crate) fn collect_pane_layout(&self) -> Option<PaneLayoutConfig> {
        fn walk(
            state: &pane_grid::State<PaneKind>,
            node: &pane_grid::Node,
        ) -> Option<PaneLayoutConfig> {
            match node {
                pane_grid::Node::Split {
                    axis, ratio, a, b, ..
                } => {
                    let axis = match axis {
                        pane_grid::Axis::Horizontal => AxisConfig::Horizontal,
                        pane_grid::Axis::Vertical => AxisConfig::Vertical,
                    };
                    Some(PaneLayoutConfig::Split {
                        axis,
                        ratio: *ratio,
                        a: Box::new(walk(state, a)?),
                        b: Box::new(walk(state, b)?),
                    })
                }
                pane_grid::Node::Pane(pane) => state
                    .get(*pane)
                    .map(pane_kind_to_config)
                    .map(PaneLayoutConfig::Leaf),
            }
        }

        walk(&self.panes, self.panes.layout())
    }

    /// Extract the current pane layout split ratios by walking the Node tree
    /// in pre-order (matching the order used by boot()'s Configuration).
    pub(crate) fn collect_layout_ratios(&self) -> Vec<f32> {
        fn walk(node: &pane_grid::Node, ratios: &mut Vec<f32>) {
            if let pane_grid::Node::Split { ratio, a, b, .. } = node {
                ratios.push(*ratio);
                walk(a, ratios);
                walk(b, ratios);
            }
        }
        let mut ratios = Vec::new();
        walk(self.panes.layout(), &mut ratios);
        ratios
    }
}
