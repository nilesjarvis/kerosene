use super::*;
use crate::annotations::{AnnotationStyle, FibKind};
use crate::chart_state::ChartInstance;
use crate::timeframe::Timeframe;

#[test]
fn workspace_snapshot_exposes_selected_chart_and_safe_indicator_catalog() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.charts.clear();
    let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    chart.macro_indicators.tf_ema_50 = true;
    chart.chart.macro_indicators = chart.macro_indicators.clone();
    chart.annotations.push(Annotation {
        id: 42,
        kind: AnnotationKind::Fib {
            kind: FibKind::Retracement,
            points: vec![(1_700_000_000_000, 50_000.0), (1_700_003_600_000, 60_000.0)],
        },
        style: AnnotationStyle {
            label: Some("Primary swing drawing-secret".to_string()),
            locked: true,
            ..AnnotationStyle::default()
        },
    });
    chart.selected_annotation = Some(42);
    terminal.charts.insert(7, chart);
    terminal.primary_chart_id = Some(7);
    terminal.hydromancer_api_key = String::new().into();
    terminal.openrouter_api_key = "drawing-secret".into();

    let bytes = terminal.build_agent_snapshot().expect("snapshot");
    let value: Value = serde_json::from_slice(&bytes).expect("json");
    assert!(!String::from_utf8_lossy(&bytes).contains("drawing-secret"));
    let workspace = &value["workspace"];
    let catalog = workspace["indicator_catalog"]
        .as_array()
        .expect("indicator catalog");
    let drawing_types = workspace["drawing_catalog"]["types"]
        .as_array()
        .expect("drawing catalog");

    assert_eq!(workspace["selected_chart_id"], 7);
    assert_eq!(workspace["charts"][0]["id"], 7);
    assert_eq!(workspace["charts"][0]["selected"], true);
    assert_eq!(workspace["charts"][0]["symbol"], "BTC");
    assert_eq!(workspace["charts"][0]["timeframe"], "1H");
    assert_eq!(workspace["charts"][0]["indicators"]["tf_ema_50"], true);
    assert!(catalog.iter().any(|entry| entry["id"] == "tf_ema_50"));
    assert!(catalog.iter().any(|entry| {
        entry["id"] == "funding_rate"
            && entry["available"] == false
            && entry["unavailable_reason"].is_string()
    }));
    assert!(!catalog.iter().any(|entry| entry["id"] == "quick_trade"));
    assert!(!catalog.iter().any(|entry| entry["id"] == "labels"));
    assert_eq!(drawing_types.len(), 9);
    assert!(
        drawing_types
            .iter()
            .any(|entry| entry["id"] == "fib_extension" && entry["anchor_count"] == 3)
    );
    assert_eq!(workspace["charts"][0]["selected_drawing_id"], 42);
    assert_eq!(workspace["charts"][0]["drawings"][0]["id"], 42);
    assert_eq!(
        workspace["charts"][0]["drawings"][0]["type"],
        "fib_retracement"
    );
    assert_eq!(
        workspace["charts"][0]["drawings"][0]["geometry"]["points"][1]["price"],
        60_000.0
    );
    assert_eq!(
        workspace["charts"][0]["drawings"][0]["style"]["label"],
        "Primary swing <redacted>"
    );
    assert_eq!(
        workspace["charts"][0]["drawings"][0]["style"]["label_truncated_or_sanitized"],
        true
    );
    assert_eq!(
        workspace["charts"][0]["drawings"][0]["style"]["locked"],
        true
    );
    assert_eq!(
        workspace["drawing_coverage"]["complete_for_current_state"],
        true
    );
}

#[test]
fn workspace_drawing_snapshot_is_bounded_and_prioritizes_selection() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.charts.clear();
    let mut chart = ChartInstance::new(7, "BTC".to_string(), Timeframe::H1);
    for id in 0..=MAX_WORKSPACE_DRAWINGS as u64 {
        chart.annotations.push(Annotation {
            id,
            kind: AnnotationKind::HorizontalLevel {
                price: 50_000.0 + id as f64,
            },
            style: AnnotationStyle::default(),
        });
    }
    chart.selected_annotation = Some(MAX_WORKSPACE_DRAWINGS as u64);
    terminal.charts.insert(7, chart);
    terminal.primary_chart_id = Some(7);

    let bytes = terminal.build_agent_snapshot().expect("snapshot");
    let value: Value = serde_json::from_slice(&bytes).expect("json");
    let workspace = &value["workspace"];
    let drawings = workspace["charts"][0]["drawings"]
        .as_array()
        .expect("drawings");

    assert_eq!(drawings.len(), MAX_WORKSPACE_DRAWINGS);
    assert_eq!(drawings[0]["id"], MAX_WORKSPACE_DRAWINGS as u64);
    assert_eq!(drawings[0]["selected"], true);
    assert_eq!(
        workspace["charts"][0]["drawing_coverage"]["truncated"],
        true
    );
    assert_eq!(
        workspace["drawing_coverage"]["returned_count"],
        MAX_WORKSPACE_DRAWINGS
    );
    assert_eq!(
        workspace["drawing_coverage"]["total_count"],
        MAX_WORKSPACE_DRAWINGS + 1
    );
    assert_eq!(workspace["drawing_coverage"]["truncated"], true);
}

#[test]
fn workspace_snapshot_is_bounded_and_keeps_the_selected_chart() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.charts.clear();
    for id in 1..=MAX_WORKSPACE_CHARTS as u64 + 3 {
        terminal.charts.insert(
            id,
            ChartInstance::new(id, format!("ASSET{id}"), Timeframe::H1),
        );
    }
    terminal.primary_chart_id = Some(MAX_WORKSPACE_CHARTS as u64 + 3);

    let bytes = terminal.build_agent_snapshot().expect("snapshot");
    let value: Value = serde_json::from_slice(&bytes).expect("json");
    let workspace = &value["workspace"];
    let charts = workspace["charts"].as_array().expect("charts");

    assert_eq!(charts.len(), MAX_WORKSPACE_CHARTS);
    assert!(charts.iter().any(|chart| chart["selected"] == true));
    assert_eq!(
        workspace["coverage"]["returned_count"],
        MAX_WORKSPACE_CHARTS
    );
    assert_eq!(
        workspace["coverage"]["total_count"],
        MAX_WORKSPACE_CHARTS + 3
    );
    assert_eq!(workspace["coverage"]["truncated"], true);
    assert_eq!(workspace["coverage"]["complete_for_current_state"], false);
}

#[test]
fn pen_snapshot_bounds_geometry_and_reports_truncation() {
    let annotation = Annotation {
        id: 7,
        kind: AnnotationKind::Pen {
            points: (0..100)
                .map(|index| (index, 100.0 + index as f64))
                .collect(),
        },
        style: AnnotationStyle::default(),
    };
    let snapshot = agent_drawing_snapshot(&annotation, true, (None, false));
    assert_eq!(snapshot["type"], "pen");
    assert_eq!(snapshot["geometry"]["total_points"], 100);
    assert_eq!(snapshot["geometry"]["points_truncated"], true);
    assert_eq!(
        snapshot["geometry"]["points"]
            .as_array()
            .expect("points")
            .len(),
        MAX_WORKSPACE_PEN_POINTS
    );
}
