use super::{
    MAX_WORKSPACE_CHARTS, MAX_WORKSPACE_DRAWING_LABEL_CHARS, MAX_WORKSPACE_DRAWINGS,
    MAX_WORKSPACE_PEN_POINTS, section_provenance,
};
use crate::agent_workspace::{ASSISTANT_DRAWING_CATALOG, annotation_kind_key};
use crate::annotations::{Annotation, AnnotationKind, LineStyle};
use crate::app_state::TradingTerminal;
use crate::chart_indicator::ChartIndicatorId;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Assistant Chart Workspace Snapshot
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn agent_workspace_snapshot(&self, generated_at_ms: u64) -> Value {
        let selected_chart_id = self
            .primary_chart_id
            .filter(|id| self.charts.contains_key(id));
        let total_chart_count = self.charts.len();
        let total_drawing_count = self
            .charts
            .values()
            .map(|instance| instance.annotations.len())
            .sum::<usize>();
        let mut drawing_budget = MAX_WORKSPACE_DRAWINGS;
        let mut returned_drawing_count = 0;
        let mut chart_instances = self.charts.values().collect::<Vec<_>>();
        chart_instances
            .sort_by_key(|instance| (selected_chart_id != Some(instance.id), instance.id));
        let mut charts = chart_instances
            .into_iter()
            .take(MAX_WORKSPACE_CHARTS)
            .map(|instance| {
                let indicators = ChartIndicatorId::ASSISTANT_VISIBLE
                    .iter()
                    .map(|indicator| {
                        (
                            indicator.key().to_string(),
                            Value::Bool(indicator.is_enabled(instance)),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>();
                let moving_average_periods = ChartIndicatorId::MOVING_AVERAGES
                    .into_iter()
                    .filter_map(|key| key.period(&instance.macro_indicators)
                        .map(|period| (key.key().to_string(), json!(period))))
                    .collect::<serde_json::Map<_, _>>();
                let total_chart_drawings = instance.annotations.len();
                let mut drawing_refs = instance.annotations.iter().collect::<Vec<_>>();
                drawing_refs.sort_by_key(|annotation| {
                    (
                        instance.selected_annotation != Some(annotation.id),
                        annotation.id,
                    )
                });
                let drawings = drawing_refs
                    .into_iter()
                    .take(drawing_budget)
                    .map(|annotation| {
                        agent_drawing_snapshot(
                            annotation,
                            instance.selected_annotation == Some(annotation.id),
                            annotation
                                .style
                                .label
                                .as_deref()
                                .map(|label| self.sanitized_agent_drawing_label(label))
                                .unwrap_or((None, false)),
                        )
                    })
                    .collect::<Vec<_>>();
                let returned_chart_drawings = drawings.len();
                drawing_budget = drawing_budget.saturating_sub(returned_chart_drawings);
                returned_drawing_count += returned_chart_drawings;
                json!({
                    "id": instance.id,
                    "surface": if self.chart_is_docked(instance.id) { "docked" } else { "detached" },
                    "symbol": instance.symbol,
                    "display_symbol": instance.symbol_display,
                    "timeframe": instance.interval.label(),
                    "timeframe_config": instance.interval.config_str(),
                    "selected": selected_chart_id == Some(instance.id),
                    "indicators": indicators,
                    "moving_average_periods": moving_average_periods,
                    "selected_drawing_id": instance.selected_annotation,
                    "drawings": drawings,
                    "drawing_coverage": {
                        "returned_count": returned_chart_drawings,
                        "total_count": total_chart_drawings,
                        "truncated": returned_chart_drawings < total_chart_drawings,
                        "complete_for_current_state": returned_chart_drawings == total_chart_drawings,
                    },
                })
            })
            .collect::<Vec<_>>();
        charts.sort_by_key(|chart| chart.get("id").and_then(Value::as_u64).unwrap_or_default());
        let returned_chart_count = charts.len();

        let indicator_catalog = ChartIndicatorId::ASSISTANT_VISIBLE
            .iter()
            .map(|indicator| {
                let available = !indicator.requires_hydromancer()
                    || !self.hydromancer_api_key.trim().is_empty();
                json!({
                    "id": indicator.key(),
                    "label": indicator.label(),
                    "group": indicator.group(),
                    "aliases": indicator.aliases(),
                    "available": available,
                    "unavailable_reason": (!available).then_some(
                        "Requires a Hydromancer API key in Settings > Integrations"
                    ),
                    "persisted": true,
                })
            })
            .collect::<Vec<_>>();
        let drawing_catalog = ASSISTANT_DRAWING_CATALOG
            .iter()
            .map(|(id, label, anchor_count)| {
                json!({
                    "id": id,
                    "label": label,
                    "anchor_count": anchor_count,
                })
            })
            .collect::<Vec<_>>();

        json!({
            "provenance": section_provenance(
                "kerosene_open_chart_state",
                Some(generated_at_ms),
                generated_at_ms,
                Some(0),
            ),
            "selected_chart_id": selected_chart_id,
            "charts": charts,
            "indicator_catalog": indicator_catalog,
            "drawing_catalog": {
                "types": drawing_catalog,
                "colors": ["blue", "yellow", "teal", "red", "purple", "white"],
                "widths": [1.0, 1.5, 2.5, 4.0],
                "line_styles": ["solid", "dashed", "dotted"],
                "maximum_label_characters": 80,
                "coordinate_contract": "time_ms is Unix epoch milliseconds; prices must be finite and greater than zero",
            },
            "action_policy": {
                "set_chart_indicators_available": true,
                "manage_chart_drawings_available": true,
                "scope": "allowlisted reversible visual settings and persisted annotations on already-open candlestick charts",
                "excluded": [
                    "orders",
                    "signing",
                    "indicator_presentation_labels",
                    "quick_trade_controls",
                    "arbitrary_indicator_code",
                    "chart_creation",
                    "symbol_changes",
                    "timeframe_changes",
                    "drawing_geometry_edits",
                    "drawing_style_edits"
                ]
            },
            "coverage": {
                "returned_count": returned_chart_count,
                "total_count": total_chart_count,
                "truncated": returned_chart_count < total_chart_count,
                "complete_for_current_state": returned_chart_count == total_chart_count,
            },
            "drawing_coverage": {
                "returned_count": returned_drawing_count,
                "total_count": total_drawing_count,
                "truncated": returned_drawing_count < total_drawing_count,
                "complete_for_current_state": returned_drawing_count == total_drawing_count,
            }
        })
    }

    fn sanitized_agent_drawing_label(&self, label: &str) -> (Option<String>, bool) {
        let sanitized = self.sanitized_journal_text(label);
        let redacted = sanitized != label;
        let (label, bounded_or_sanitized) = bounded_agent_drawing_label(&sanitized);
        (label, redacted || bounded_or_sanitized)
    }
}

fn agent_drawing_snapshot(
    annotation: &Annotation,
    selected: bool,
    label: (Option<String>, bool),
) -> Value {
    let geometry = match &annotation.kind {
        AnnotationKind::Pen { points } => json!({
            "points": points.iter().take(MAX_WORKSPACE_PEN_POINTS).copied().map(agent_drawing_anchor).collect::<Vec<_>>(),
            "total_points": points.len(),
            "points_truncated": points.len() > MAX_WORKSPACE_PEN_POINTS,
        }),
        AnnotationKind::HorizontalLevel { price } => json!({ "price": price }),
        AnnotationKind::VerticalLine { time } => json!({ "time_ms": time }),
        AnnotationKind::TrendLine { start, end }
        | AnnotationKind::Ray { start, end }
        | AnnotationKind::ExtendedLine { start, end }
        | AnnotationKind::Measure { start, end } => json!({
            "start": agent_drawing_anchor(*start),
            "end": agent_drawing_anchor(*end),
        }),
        AnnotationKind::Rectangle { a, b } => json!({
            "a": agent_drawing_anchor(*a),
            "b": agent_drawing_anchor(*b),
        }),
        AnnotationKind::Fib { points, .. } => json!({
            "points": points
                .iter()
                .copied()
                .map(agent_drawing_anchor)
                .collect::<Vec<_>>(),
        }),
    };
    let (label, label_truncated_or_sanitized) = label;

    json!({
        "id": annotation.id,
        "type": annotation_kind_key(&annotation.kind),
        "geometry": geometry,
        "style": {
            "color_rgba": [
                annotation.style.color.r,
                annotation.style.color.g,
                annotation.style.color.b,
                annotation.style.color.a,
            ],
            "width": annotation.style.width,
            "line_style": match annotation.style.line_style {
                LineStyle::Solid => "solid",
                LineStyle::Dashed => "dashed",
                LineStyle::Dotted => "dotted",
            },
            "label": label,
            "label_truncated_or_sanitized": label_truncated_or_sanitized,
            "locked": annotation.style.locked,
            "visible": annotation.style.visible,
        },
        "selected": selected,
    })
}

fn agent_drawing_anchor((time_ms, price): (u64, f64)) -> Value {
    json!({ "time_ms": time_ms, "price": price })
}

fn bounded_agent_drawing_label(label: &str) -> (Option<String>, bool) {
    let mut changed = false;
    let mut bounded = String::new();
    for (count, character) in label.chars().enumerate() {
        if count >= MAX_WORKSPACE_DRAWING_LABEL_CHARS {
            changed = true;
            break;
        }
        if character.is_control() {
            bounded.push(' ');
            changed = true;
        } else {
            bounded.push(character);
        }
    }
    let bounded = bounded.trim().to_string();
    if bounded.is_empty() {
        (None, changed)
    } else {
        (Some(bounded), changed)
    }
}

#[cfg(test)]
mod tests;
