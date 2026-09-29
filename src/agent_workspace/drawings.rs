use super::drawing_model::{ChartDrawingOperation, annotation_kind_key};
use super::{AgentHostActionError, MAX_DRAWING_OPERATIONS};
use crate::annotations::{Annotation, AnnotationId};
use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::Task;
use serde::Serialize;
use std::collections::{HashMap, HashSet, hash_map::Entry};

// ---------------------------------------------------------------------------
// Assistant Drawing Batches
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AgentChartDrawingResponse {
    success: bool,
    action: &'static str,
    operations: Vec<ChartDrawingOperationResult>,
    persistence_scheduled: bool,
    warnings: Vec<String>,
    error: Option<AgentHostActionError>,
}

#[derive(Serialize)]
struct ChartDrawingOperationResult {
    operation_index: usize,
    chart_id: ChartId,
    symbol: String,
    display_symbol: String,
    timeframe: String,
    drawing_id: AnnotationId,
    drawing_type: &'static str,
    outcome: &'static str,
}

struct DrawingChartCandidate {
    annotations: Vec<Annotation>,
    next_annotation_id: AnnotationId,
    selected_annotation: Option<AnnotationId>,
}

impl AgentChartDrawingResponse {
    fn failure(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            action: "manage_chart_drawings",
            operations: Vec::new(),
            persistence_scheduled: false,
            warnings: Vec::new(),
            error: Some(AgentHostActionError {
                code,
                message: message.into(),
            }),
        }
    }

    fn into_json(self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| {
            r#"{"success":false,"action":"manage_chart_drawings","operations":[],"persistence_scheduled":false,"warnings":[],"error":{"code":"response_serialization_failed","message":"Kerosene could not serialize the workspace action result"}}"#.to_string()
        })
    }
}

impl TradingTerminal {
    pub(super) fn apply_agent_chart_drawing_operations(
        &mut self,
        operations: Vec<ChartDrawingOperation>,
    ) -> (String, Task<Message>) {
        if operations.is_empty() || operations.len() > MAX_DRAWING_OPERATIONS {
            return drawing_failure_result(
                "invalid_operation_count",
                format!("Choose between 1 and {MAX_DRAWING_OPERATIONS} drawing operations"),
            );
        }

        let mut candidates = HashMap::<ChartId, DrawingChartCandidate>::new();
        for chart_id in operations.iter().map(ChartDrawingOperation::chart_id) {
            if let Entry::Vacant(candidate) = candidates.entry(chart_id) {
                let Some(instance) = self.charts.get(&chart_id) else {
                    return drawing_failure_result(
                        "chart_not_found",
                        format!("Chart {chart_id} is no longer open"),
                    );
                };
                candidate.insert(DrawingChartCandidate {
                    annotations: instance.annotations.clone(),
                    next_annotation_id: instance.next_annotation_id,
                    selected_annotation: instance.selected_annotation,
                });
            }
        }

        let mut removed_ids = HashSet::new();
        let mut results = Vec::with_capacity(operations.len());
        let mut changed_any = false;
        for (operation_index, operation) in operations.into_iter().enumerate() {
            let chart_id = operation.chart_id();
            let Some(candidate) = candidates.get_mut(&chart_id) else {
                return drawing_failure_result(
                    "chart_not_found",
                    format!("Chart {chart_id} is no longer open"),
                );
            };
            let (drawing_id, drawing_type, outcome) = match operation {
                ChartDrawingOperation::Add { drawing, .. } => {
                    let mut annotation = match drawing.into_annotation() {
                        Ok(annotation) => annotation,
                        Err(message) => {
                            return drawing_failure_result("invalid_drawing", message);
                        }
                    };
                    let drawing_type = annotation_kind_key(&annotation.kind);
                    if let Some(existing) = candidate.annotations.iter().find(|existing| {
                        existing.kind == annotation.kind && existing.style == annotation.style
                    }) {
                        (existing.id, drawing_type, "already_present")
                    } else {
                        let Some(drawing_id) = next_available_annotation_id(candidate) else {
                            return drawing_failure_result(
                                "drawing_id_exhausted",
                                format!("Chart {chart_id} cannot allocate another drawing ID"),
                            );
                        };
                        annotation.id = drawing_id;
                        candidate.annotations.push(annotation);
                        changed_any = true;
                        (drawing_id, drawing_type, "created")
                    }
                }
                ChartDrawingOperation::Remove { drawing_id, .. } => {
                    if !removed_ids.insert((chart_id, drawing_id)) {
                        return drawing_failure_result(
                            "duplicate_remove",
                            format!(
                                "Drawing {drawing_id} on chart {chart_id} may be removed only once per action"
                            ),
                        );
                    }
                    let Some(index) = candidate
                        .annotations
                        .iter()
                        .position(|annotation| annotation.id == drawing_id)
                    else {
                        return drawing_failure_result(
                            "drawing_not_found",
                            format!("Drawing {drawing_id} is no longer on chart {chart_id}"),
                        );
                    };
                    if candidate.annotations[index].style.locked {
                        return drawing_failure_result(
                            "drawing_locked",
                            format!(
                                "Drawing {drawing_id} on chart {chart_id} is locked; unlock it before removal"
                            ),
                        );
                    }
                    let drawing_type = annotation_kind_key(&candidate.annotations[index].kind);
                    candidate.annotations.remove(index);
                    changed_any = true;
                    if candidate.selected_annotation == Some(drawing_id) {
                        candidate.selected_annotation = None;
                    }
                    (drawing_id, drawing_type, "removed")
                }
            };

            let Some(instance) = self.charts.get(&chart_id) else {
                return drawing_failure_result(
                    "chart_not_found",
                    format!("Chart {chart_id} is no longer open"),
                );
            };
            results.push(ChartDrawingOperationResult {
                operation_index,
                chart_id,
                symbol: instance.symbol.clone(),
                display_symbol: instance.symbol_display.clone(),
                timeframe: instance.interval.label().to_string(),
                drawing_id,
                drawing_type,
                outcome,
            });
        }

        for (chart_id, candidate) in candidates {
            let Some(instance) = self.charts.get_mut(&chart_id) else {
                return drawing_failure_result(
                    "chart_not_found",
                    format!("Chart {chart_id} is no longer open"),
                );
            };
            instance.annotations = candidate.annotations;
            instance.next_annotation_id = candidate.next_annotation_id;
            instance.selected_annotation = candidate.selected_annotation;
            instance.chart.annotations = instance.annotations.clone();
        }

        let persistence_scheduled = self.persist_agent_workspace_changes(changed_any);
        let warnings = if changed_any && !persistence_scheduled {
            vec![
                "Drawing changes are active for this session, but configuration persistence is paused"
                    .to_string(),
            ]
        } else {
            Vec::new()
        };

        (
            AgentChartDrawingResponse {
                success: true,
                action: "manage_chart_drawings",
                operations: results,
                persistence_scheduled,
                warnings,
                error: None,
            }
            .into_json(),
            Task::none(),
        )
    }
}

fn next_available_annotation_id(candidate: &mut DrawingChartCandidate) -> Option<AnnotationId> {
    let mut drawing_id = candidate.next_annotation_id;
    while candidate
        .annotations
        .iter()
        .any(|annotation| annotation.id == drawing_id)
    {
        drawing_id = drawing_id.checked_add(1)?;
    }
    candidate.next_annotation_id = drawing_id.checked_add(1)?;
    Some(drawing_id)
}

pub(super) fn drawing_failure_result(
    code: &'static str,
    message: impl Into<String>,
) -> (String, Task<Message>) {
    (
        AgentChartDrawingResponse::failure(code, message).into_json(),
        Task::none(),
    )
}
