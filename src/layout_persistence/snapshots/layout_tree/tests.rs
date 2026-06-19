use super::*;
use crate::config::{AxisConfig, PaneKindConfig, PaneLayoutConfig};

#[test]
fn pane_layout_conversion_prunes_unsupported_sibling() {
    let layout = PaneLayoutConfig::Split {
        axis: AxisConfig::Vertical,
        ratio: 0.4,
        a: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Chart {
            chart_id: 42,
        })),
        b: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Unsupported)),
    };

    let config = TradingTerminal::pane_layout_to_configuration(&layout).expect("supported sibling");

    assert!(matches!(
        config,
        pane_grid::Configuration::Pane(PaneKind::Chart(42))
    ));
}

#[test]
fn pane_layout_conversion_drops_unsupported_only_layout() {
    let layout = PaneLayoutConfig::Leaf(PaneKindConfig::Unsupported);

    assert!(TradingTerminal::pane_layout_to_configuration(&layout).is_none());
}

#[test]
fn pane_layout_conversion_prunes_unknown_future_sibling() {
    let layout = PaneLayoutConfig::Split {
        axis: AxisConfig::Vertical,
        ratio: 0.4,
        a: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Chart {
            chart_id: 42,
        })),
        b: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Unknown(
            serde_json::json!({ "FuturePane": { "id": 1 } }),
        ))),
    };

    let config = TradingTerminal::pane_layout_to_configuration(&layout).expect("supported sibling");

    assert!(matches!(
        config,
        pane_grid::Configuration::Pane(PaneKind::Chart(42))
    ));
}

#[test]
fn pane_layout_conversion_prunes_legacy_account_summary_sibling() {
    let layout = PaneLayoutConfig::Split {
        axis: AxisConfig::Horizontal,
        ratio: 0.06,
        a: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::AccountSummary)),
        b: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Chart {
            chart_id: 7,
        })),
    };

    let config =
        TradingTerminal::pane_layout_to_configuration(&layout).expect("movable pane sibling");

    assert!(matches!(
        config,
        pane_grid::Configuration::Pane(PaneKind::Chart(7))
    ));
}

#[test]
fn pane_layout_conversion_normalizes_split_ratio() {
    let layout = PaneLayoutConfig::Split {
        axis: AxisConfig::Vertical,
        ratio: f32::NAN,
        a: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Chart {
            chart_id: 42,
        })),
        b: Box::new(PaneLayoutConfig::Leaf(PaneKindConfig::Watchlist)),
    };

    let config = TradingTerminal::pane_layout_to_configuration(&layout).expect("valid layout");

    match config {
        pane_grid::Configuration::Split { ratio, .. } => {
            assert_eq!(ratio, 0.5);
        }
        other => panic!("expected split configuration, got {other:?}"),
    }
}
