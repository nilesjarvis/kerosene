use crate::annotations::{
    Annotation, AnnotationId, AnnotationKind, AnnotationStyle, DEFAULT_LEVEL_COLOR,
    DEFAULT_LINE_COLOR, DEFAULT_MEASURE_COLOR, DrawingTool, FibKind, LineStyle,
};
use crate::app_state::TradingTerminal;
use crate::chart_indicator::ChartIndicatorId;
use crate::chart_state::ChartId;
use crate::message::Message;

use iced::{Color, Task};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const HOST_ACTION_VERSION: u32 = 1;
const MAX_HOST_ACTION_BYTES: usize = 64 * 1024;
const MAX_TARGET_CHARTS: usize = 32;
const MAX_INDICATOR_CHANGES: usize = 32;
const MAX_INDICATOR_APPLICATIONS: usize = MAX_TARGET_CHARTS * MAX_INDICATOR_CHANGES;
const MAX_DRAWING_OPERATIONS: usize = 64;
const MAX_DRAWING_LABEL_CHARS: usize = 80;
pub(crate) const HOST_ACTION_RPC_TITLE: &str = "KEROSENE_HOST_ACTION_V1";
pub(crate) const ASSISTANT_DRAWING_CATALOG: [(&str, &str, usize); 9] = [
    ("horizontal_level", "Horizontal level", 1),
    ("vertical_line", "Vertical line", 1),
    ("trend_line", "Trend line", 2),
    ("ray", "Ray", 2),
    ("extended_line", "Extended line", 2),
    ("rectangle", "Rectangle / zone", 2),
    ("measure", "Price / time measurement", 2),
    ("fib_retracement", "Fibonacci retracement", 2),
    ("fib_extension", "Fibonacci extension", 3),
];

// ---------------------------------------------------------------------------
// Assistant Workspace Action Contract
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentHostActionRequest {
    version: u32,
    tool_call_id: String,
    action: AgentWorkspaceAction,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum AgentWorkspaceAction {
    SetChartIndicators {
        chart_ids: Vec<ChartId>,
        changes: Vec<ChartIndicatorChange>,
    },
    ManageChartDrawings {
        operations: Vec<ChartDrawingOperation>,
    },
}

impl AgentWorkspaceAction {
    fn tool_name(&self) -> &'static str {
        match self {
            Self::SetChartIndicators { .. } => "kerosene_set_chart_indicators",
            Self::ManageChartDrawings { .. } => "kerosene_manage_chart_drawings",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChartIndicatorChange {
    indicator_id: ChartIndicatorId,
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum ChartDrawingOperation {
    Add {
        chart_id: ChartId,
        drawing: ChartDrawingSpec,
    },
    Remove {
        chart_id: ChartId,
        drawing_id: AnnotationId,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ChartDrawingSpec {
    HorizontalLevel {
        price: f64,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    VerticalLine {
        time_ms: u64,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    TrendLine {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    Ray {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    ExtendedLine {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    Rectangle {
        a: AgentDrawingAnchor,
        b: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    Measure {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    FibRetracement {
        a: AgentDrawingAnchor,
        b: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    FibExtension {
        a: AgentDrawingAnchor,
        b: AgentDrawingAnchor,
        c: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
}

impl ChartDrawingSpec {
    fn type_key(&self) -> &'static str {
        match self {
            Self::HorizontalLevel { .. } => "horizontal_level",
            Self::VerticalLine { .. } => "vertical_line",
            Self::TrendLine { .. } => "trend_line",
            Self::Ray { .. } => "ray",
            Self::ExtendedLine { .. } => "extended_line",
            Self::Rectangle { .. } => "rectangle",
            Self::Measure { .. } => "measure",
            Self::FibRetracement { .. } => "fib_retracement",
            Self::FibExtension { .. } => "fib_extension",
        }
    }

    fn into_annotation(self) -> Result<Annotation, &'static str> {
        let (kind, tool, style) = match self {
            Self::HorizontalLevel { price, style } => (
                AnnotationKind::HorizontalLevel { price },
                DrawingTool::HorizontalLevel,
                style,
            ),
            Self::VerticalLine { time_ms, style } => (
                AnnotationKind::VerticalLine { time: time_ms },
                DrawingTool::VerticalLine,
                style,
            ),
            Self::TrendLine { start, end, style } => (
                AnnotationKind::TrendLine {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::TrendLine,
                style,
            ),
            Self::Ray { start, end, style } => (
                AnnotationKind::Ray {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::Ray,
                style,
            ),
            Self::ExtendedLine { start, end, style } => (
                AnnotationKind::ExtendedLine {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::ExtendedLine,
                style,
            ),
            Self::Rectangle { a, b, style } => (
                AnnotationKind::Rectangle {
                    a: a.into_anchor(),
                    b: b.into_anchor(),
                },
                DrawingTool::Rectangle,
                style,
            ),
            Self::Measure { start, end, style } => (
                AnnotationKind::Measure {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::Measure,
                style,
            ),
            Self::FibRetracement { a, b, style } => (
                AnnotationKind::Fib {
                    kind: FibKind::Retracement,
                    points: vec![a.into_anchor(), b.into_anchor()],
                },
                DrawingTool::FibRetracement,
                style,
            ),
            Self::FibExtension { a, b, c, style } => (
                AnnotationKind::Fib {
                    kind: FibKind::Extension,
                    points: vec![a.into_anchor(), b.into_anchor(), c.into_anchor()],
                },
                DrawingTool::FibExtension,
                style,
            ),
        };
        let mut annotation = Annotation {
            id: 0,
            kind,
            style: AnnotationStyle::for_tool(tool),
        };
        style.apply_to(&mut annotation.style)?;
        annotation
            .is_valid()
            .then_some(annotation)
            .ok_or("The drawing contains invalid time or price coordinates")
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentDrawingAnchor {
    time_ms: u64,
    price: f64,
}

impl AgentDrawingAnchor {
    fn into_anchor(self) -> (u64, f64) {
        (self.time_ms, self.price)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentDrawingStyle {
    color: Option<AgentDrawingColor>,
    width: Option<f32>,
    line_style: Option<AgentDrawingLineStyle>,
    label: Option<String>,
}

impl AgentDrawingStyle {
    fn apply_to(self, target: &mut AnnotationStyle) -> Result<(), &'static str> {
        if let Some(color) = self.color {
            target.color = color.color();
        }
        if let Some(width) = self.width {
            if ![1.0, 1.5, 2.5, 4.0].contains(&width) {
                return Err("Drawing width must be one of 1, 1.5, 2.5, or 4");
            }
            target.width = width;
        }
        if let Some(line_style) = self.line_style {
            target.line_style = line_style.into();
        }
        if let Some(label) = self.label {
            let label = label.trim();
            if label.is_empty()
                || label.chars().count() > MAX_DRAWING_LABEL_CHARS
                || label.chars().any(char::is_control)
            {
                return Err("Drawing labels must contain 1 to 80 printable characters");
            }
            target.label = Some(label.to_string());
        }
        target.locked = false;
        target.visible = true;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentDrawingColor {
    Blue,
    Yellow,
    Teal,
    Red,
    Purple,
    White,
}

impl AgentDrawingColor {
    fn color(self) -> Color {
        match self {
            Self::Blue => DEFAULT_LEVEL_COLOR,
            Self::Yellow => DEFAULT_LINE_COLOR,
            Self::Teal => DEFAULT_MEASURE_COLOR,
            Self::Red => Color::from_rgb(0.95, 0.45, 0.45),
            Self::Purple => Color::from_rgb(0.62, 0.55, 0.95),
            Self::White => Color::from_rgb(0.92, 0.92, 0.92),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentDrawingLineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl From<AgentDrawingLineStyle> for LineStyle {
    fn from(style: AgentDrawingLineStyle) -> Self {
        match style {
            AgentDrawingLineStyle::Solid => Self::Solid,
            AgentDrawingLineStyle::Dashed => Self::Dashed,
            AgentDrawingLineStyle::Dotted => Self::Dotted,
        }
    }
}

#[derive(Serialize)]
struct AgentHostActionResponse {
    success: bool,
    action: &'static str,
    charts: Vec<ChartIndicatorChartResult>,
    persistence_scheduled: bool,
    warnings: Vec<String>,
    error: Option<AgentHostActionError>,
}

#[derive(Serialize)]
struct AgentHostActionError {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct ChartIndicatorChartResult {
    chart_id: ChartId,
    symbol: String,
    display_symbol: String,
    timeframe: String,
    changes: Vec<ChartIndicatorChangeResult>,
}

#[derive(Serialize)]
struct ChartIndicatorChangeResult {
    indicator_id: &'static str,
    label: &'static str,
    previous_enabled: bool,
    enabled: bool,
    outcome: &'static str,
}

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

impl AgentHostActionResponse {
    fn failure(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            action: "set_chart_indicators",
            charts: Vec::new(),
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
            r#"{"success":false,"action":"set_chart_indicators","charts":[],"persistence_scheduled":false,"warnings":[],"error":{"code":"response_serialization_failed","message":"Kerosene could not serialize the workspace action result"}}"#.to_string()
        })
    }
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
    pub(crate) fn handle_agent_host_action(&mut self, payload: &str) -> (String, Task<Message>) {
        if payload.len() > MAX_HOST_ACTION_BYTES {
            return (
                AgentHostActionResponse::failure(
                    "request_too_large",
                    "The workspace action request exceeded Kerosene's size limit",
                )
                .into_json(),
                Task::none(),
            );
        }

        let request = match serde_json::from_str::<AgentHostActionRequest>(payload) {
            Ok(request) => request,
            Err(_) => {
                return (
                    AgentHostActionResponse::failure(
                        "invalid_request",
                        "The workspace action request did not match the supported contract",
                    )
                    .into_json(),
                    Task::none(),
                );
            }
        };
        let tool_name = request.action.tool_name();

        if request.version != HOST_ACTION_VERSION {
            return action_failure_result(
                tool_name,
                "unsupported_version",
                "The workspace action contract version is not supported",
            );
        }
        if request.tool_call_id.is_empty() || request.tool_call_id.len() > 256 {
            return action_failure_result(
                tool_name,
                "invalid_tool_call",
                "The workspace action is missing a valid tool-call identifier",
            );
        }
        if !self.agent.workspace_actions_allowed
            || !self
                .agent
                .has_running_tool_call(&request.tool_call_id, tool_name)
        {
            return action_failure_result(
                tool_name,
                "inactive_tool_call",
                "The workspace action no longer belongs to the active Assistant turn",
            );
        }

        match request.action {
            AgentWorkspaceAction::SetChartIndicators { chart_ids, changes } => {
                self.apply_agent_chart_indicator_changes(chart_ids, changes)
            }
            AgentWorkspaceAction::ManageChartDrawings { operations } => {
                self.apply_agent_chart_drawing_operations(operations)
            }
        }
    }

    fn apply_agent_chart_indicator_changes(
        &mut self,
        chart_ids: Vec<ChartId>,
        changes: Vec<ChartIndicatorChange>,
    ) -> (String, Task<Message>) {
        if chart_ids.is_empty() || chart_ids.len() > MAX_TARGET_CHARTS {
            return failure_result(
                "invalid_chart_count",
                format!("Choose between 1 and {MAX_TARGET_CHARTS} open charts"),
            );
        }
        if changes.is_empty() || changes.len() > MAX_INDICATOR_CHANGES {
            return failure_result(
                "invalid_change_count",
                format!("Choose between 1 and {MAX_INDICATOR_CHANGES} indicator changes"),
            );
        }
        if chart_ids.len().saturating_mul(changes.len()) > MAX_INDICATOR_APPLICATIONS {
            return failure_result(
                "batch_too_large",
                format!(
                    "One workspace action may apply at most {MAX_INDICATOR_APPLICATIONS} chart-indicator changes"
                ),
            );
        }

        let unique_charts = chart_ids.iter().copied().collect::<HashSet<_>>();
        if unique_charts.len() != chart_ids.len() {
            return failure_result("duplicate_chart", "Each target chart may appear only once");
        }
        let unique_indicators = changes
            .iter()
            .map(|change| change.indicator_id)
            .collect::<HashSet<_>>();
        if unique_indicators.len() != changes.len() {
            return failure_result(
                "duplicate_indicator",
                "Each indicator may appear only once in a workspace action",
            );
        }
        if let Some(indicator) = changes.iter().find_map(|change| {
            (!ChartIndicatorId::ASSISTANT_VISIBLE.contains(&change.indicator_id))
                .then_some(change.indicator_id)
        }) {
            return failure_result(
                "unsupported_indicator",
                format!("{} is not available to the Assistant", indicator.label()),
            );
        }
        if let Some(chart_id) = chart_ids
            .iter()
            .find(|chart_id| !self.charts.contains_key(chart_id))
        {
            return failure_result(
                "chart_not_found",
                format!("Chart {chart_id} is no longer open"),
            );
        }

        let funding_needed = changes.iter().any(|change| {
            change.indicator_id.requires_hydromancer()
                && change.enabled
                && chart_ids.iter().any(|chart_id| {
                    self.charts
                        .get(chart_id)
                        .is_some_and(|instance| !change.indicator_id.is_enabled(instance))
                })
        });
        if funding_needed && self.hydromancer_api_key.trim().is_empty() {
            return failure_result(
                "dependency_missing",
                "Funding rate requires a Hydromancer API key in Settings > Integrations",
            );
        }

        let mut chart_results = Vec::with_capacity(chart_ids.len());
        let mut funding_fetch_ids = Vec::new();
        let mut changed_any = false;

        for chart_id in chart_ids {
            let Some(instance) = self.charts.get_mut(&chart_id) else {
                return failure_result(
                    "chart_not_found",
                    format!("Chart {chart_id} is no longer open"),
                );
            };
            let symbol = instance.symbol.clone();
            let display_symbol = instance.symbol_display.clone();
            let timeframe = instance.interval.label().to_string();
            let mut change_results = Vec::with_capacity(changes.len());

            for change in &changes {
                let previous_enabled = change.indicator_id.is_enabled(instance);
                let changed = change.indicator_id.set_enabled(instance, change.enabled);
                changed_any |= changed;

                if change.indicator_id == ChartIndicatorId::FundingRate && changed {
                    if change.enabled {
                        funding_fetch_ids.push(chart_id);
                    } else {
                        Self::clear_funding_display(instance);
                    }
                }

                change_results.push(ChartIndicatorChangeResult {
                    indicator_id: change.indicator_id.key(),
                    label: change.indicator_id.label(),
                    previous_enabled,
                    enabled: change.enabled,
                    outcome: if changed { "changed" } else { "already_set" },
                });
            }

            instance.chart.macro_indicators = instance.macro_indicators.clone();
            instance.chart.candle_cache.clear();
            chart_results.push(ChartIndicatorChartResult {
                chart_id,
                symbol,
                display_symbol,
                timeframe,
                changes: change_results,
            });
        }

        let persistence_scheduled = if changed_any {
            self.persist_config();
            self.config_save_due_at.is_some()
                && !self.secret_migration_save_blocked
                && !self.config_clear_requested
                && !self.config_cleared_this_session
        } else {
            false
        };
        let warnings = if changed_any && !persistence_scheduled {
            vec![
                "Indicator changes are active for this session, but configuration persistence is paused"
                    .to_string(),
            ]
        } else {
            Vec::new()
        };
        let tasks = funding_fetch_ids
            .into_iter()
            .map(|chart_id| self.maybe_fetch_chart_funding(chart_id))
            .collect::<Vec<_>>();

        (
            AgentHostActionResponse {
                success: true,
                action: "set_chart_indicators",
                charts: chart_results,
                persistence_scheduled,
                warnings,
                error: None,
            }
            .into_json(),
            Task::batch(tasks),
        )
    }

    fn apply_agent_chart_drawing_operations(
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
            if candidates.contains_key(&chart_id) {
                continue;
            }
            let Some(instance) = self.charts.get(&chart_id) else {
                return drawing_failure_result(
                    "chart_not_found",
                    format!("Chart {chart_id} is no longer open"),
                );
            };
            candidates.insert(
                chart_id,
                DrawingChartCandidate {
                    annotations: instance.annotations.clone(),
                    next_annotation_id: instance.next_annotation_id,
                    selected_annotation: instance.selected_annotation,
                },
            );
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
                    let drawing_type = drawing.type_key();
                    let mut annotation = match drawing.into_annotation() {
                        Ok(annotation) => annotation,
                        Err(message) => {
                            return drawing_failure_result("invalid_drawing", message);
                        }
                    };
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

        let persistence_scheduled = if changed_any {
            self.persist_config();
            self.config_save_due_at.is_some()
                && !self.secret_migration_save_blocked
                && !self.config_clear_requested
                && !self.config_cleared_this_session
        } else {
            false
        };
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

impl ChartDrawingOperation {
    fn chart_id(&self) -> ChartId {
        match self {
            Self::Add { chart_id, .. } | Self::Remove { chart_id, .. } => *chart_id,
        }
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

pub(crate) fn annotation_kind_key(kind: &AnnotationKind) -> &'static str {
    match kind {
        AnnotationKind::HorizontalLevel { .. } => "horizontal_level",
        AnnotationKind::VerticalLine { .. } => "vertical_line",
        AnnotationKind::TrendLine { .. } => "trend_line",
        AnnotationKind::Ray { .. } => "ray",
        AnnotationKind::ExtendedLine { .. } => "extended_line",
        AnnotationKind::Rectangle { .. } => "rectangle",
        AnnotationKind::Measure { .. } => "measure",
        AnnotationKind::Fib {
            kind: FibKind::Retracement,
            ..
        } => "fib_retracement",
        AnnotationKind::Fib {
            kind: FibKind::Extension,
            ..
        } => "fib_extension",
    }
}

fn failure_result(code: &'static str, message: impl Into<String>) -> (String, Task<Message>) {
    (
        AgentHostActionResponse::failure(code, message).into_json(),
        Task::none(),
    )
}

fn drawing_failure_result(
    code: &'static str,
    message: impl Into<String>,
) -> (String, Task<Message>) {
    (
        AgentChartDrawingResponse::failure(code, message).into_json(),
        Task::none(),
    )
}

fn action_failure_result(
    tool_name: &str,
    code: &'static str,
    message: impl Into<String>,
) -> (String, Task<Message>) {
    if tool_name == "kerosene_manage_chart_drawings" {
        drawing_failure_result(code, message)
    } else {
        failure_result(code, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_state::AgentChatEntry;
    use crate::chart_state::ChartInstance;
    use crate::timeframe::Timeframe;

    fn terminal_with_running_tool() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".to_string(), Timeframe::H1));
        terminal.agent.entries.push(AgentChatEntry::Tool {
            call_id: "call-1".to_string(),
            name: "kerosene_set_chart_indicators".to_string(),
            detail: None,
            finished: false,
            is_error: false,
            expanded: true,
        });
        terminal.agent.workspace_actions_allowed = true;
        terminal
    }

    fn terminal_with_running_drawing_tool() -> TradingTerminal {
        let mut terminal = terminal_with_running_tool();
        let Some(AgentChatEntry::Tool { name, .. }) = terminal.agent.entries.last_mut() else {
            panic!("running tool entry");
        };
        *name = "kerosene_manage_chart_drawings".to_string();
        terminal
    }

    fn action_payload(enabled: bool) -> String {
        serde_json::json!({
            "version": HOST_ACTION_VERSION,
            "tool_call_id": "call-1",
            "action": {
                "type": "set_chart_indicators",
                "chart_ids": [7],
                "changes": [{ "indicator_id": "tf_ema_50", "enabled": enabled }],
            }
        })
        .to_string()
    }

    fn drawing_payload(operations: serde_json::Value) -> String {
        serde_json::json!({
            "version": HOST_ACTION_VERSION,
            "tool_call_id": "call-1",
            "action": {
                "type": "manage_chart_drawings",
                "operations": operations,
            }
        })
        .to_string()
    }

    fn assert_annotations_mirrored(chart: &ChartInstance) {
        assert_eq!(chart.chart.annotations.len(), chart.annotations.len());
        for (canvas, persisted) in chart.chart.annotations.iter().zip(&chart.annotations) {
            assert_eq!(canvas.id, persisted.id);
            assert_eq!(canvas.kind, persisted.kind);
            assert_eq!(canvas.style, persisted.style);
        }
    }

    #[test]
    fn assistant_indicator_action_is_idempotent() {
        let mut terminal = terminal_with_running_tool();
        let (first, _task) = terminal.handle_agent_host_action(&action_payload(true));
        let (second, _task) = terminal.handle_agent_host_action(&action_payload(true));
        let first: serde_json::Value = serde_json::from_str(&first).expect("first result");
        let second: serde_json::Value = serde_json::from_str(&second).expect("second result");

        assert_eq!(first["success"], true);
        assert_eq!(first["charts"][0]["changes"][0]["outcome"], "changed");
        assert_eq!(second["success"], true);
        assert_eq!(second["charts"][0]["changes"][0]["outcome"], "already_set");
        assert!(terminal.charts[&7].macro_indicators.tf_ema_50);
    }

    #[test]
    fn quick_trade_is_not_available_to_the_assistant() {
        let mut terminal = terminal_with_running_tool();
        let payload = serde_json::json!({
            "version": HOST_ACTION_VERSION,
            "tool_call_id": "call-1",
            "action": {
                "type": "set_chart_indicators",
                "chart_ids": [7],
                "changes": [{ "indicator_id": "quick_trade", "enabled": true }],
            }
        })
        .to_string();

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");
        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "unsupported_indicator");
        assert!(!terminal.charts[&7].macro_indicators.show_quick_trade);
    }

    #[test]
    fn a_failed_dependency_preflight_does_not_apply_part_of_the_batch() {
        let mut terminal = terminal_with_running_tool();
        terminal.hydromancer_api_key = String::new().into();
        let payload = serde_json::json!({
            "version": HOST_ACTION_VERSION,
            "tool_call_id": "call-1",
            "action": {
                "type": "set_chart_indicators",
                "chart_ids": [7],
                "changes": [
                    { "indicator_id": "tf_ema_50", "enabled": true },
                    { "indicator_id": "funding_rate", "enabled": true },
                ],
            }
        })
        .to_string();

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "dependency_missing");
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
        assert!(!terminal.charts[&7].macro_indicators.show_funding_rate);
    }

    #[test]
    fn stale_tool_call_cannot_mutate_a_chart() {
        let mut terminal = terminal_with_running_tool();
        let payload = action_payload(true).replace("call-1", "stale-call");
        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["action"], "set_chart_indicators");
        assert_eq!(result["error"]["code"], "inactive_tool_call");
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
    }

    #[test]
    fn aborted_turn_cannot_mutate_a_chart() {
        let mut terminal = terminal_with_running_tool();
        let _task = terminal.update_agent(Message::AgentAbort);
        let (result, _task) = terminal.handle_agent_host_action(&action_payload(true));
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "inactive_tool_call");
        assert!(
            !terminal
                .agent
                .has_running_tool_call("call-1", "kerosene_set_chart_indicators")
        );
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
    }

    #[test]
    fn unknown_action_fields_are_rejected() {
        let mut terminal = terminal_with_running_tool();
        let payload =
            action_payload(true).replace("\"changes\":[", "\"unexpected\":true,\"changes\":[");
        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "invalid_request");
        assert!(!terminal.charts[&7].macro_indicators.tf_ema_50);
    }

    #[test]
    fn assistant_can_create_every_supported_drawing_type() {
        let mut terminal = terminal_with_running_drawing_tool();
        let payload = drawing_payload(serde_json::json!([
            { "operation": "add", "chart_id": 7, "drawing": { "type": "horizontal_level", "price": 60_000.0 } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "vertical_line", "time_ms": 1_700_000_000_000_u64 } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "trend_line", "start": { "time_ms": 1_700_000_000_000_u64, "price": 59_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 61_000.0 } } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "ray", "start": { "time_ms": 1_700_000_000_000_u64, "price": 58_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 60_000.0 } } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "extended_line", "start": { "time_ms": 1_700_000_000_000_u64, "price": 57_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 59_000.0 } } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "rectangle", "a": { "time_ms": 1_700_000_000_000_u64, "price": 55_000.0 }, "b": { "time_ms": 1_700_003_600_000_u64, "price": 56_000.0 }, "style": { "color": "purple", "width": 2.5, "line_style": "dashed", "label": "Demand zone" } } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "measure", "start": { "time_ms": 1_700_000_000_000_u64, "price": 60_000.0 }, "end": { "time_ms": 1_700_003_600_000_u64, "price": 62_000.0 } } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "fib_retracement", "a": { "time_ms": 1_700_000_000_000_u64, "price": 50_000.0 }, "b": { "time_ms": 1_700_003_600_000_u64, "price": 65_000.0 } } },
            { "operation": "add", "chart_id": 7, "drawing": { "type": "fib_extension", "a": { "time_ms": 1_700_000_000_000_u64, "price": 50_000.0 }, "b": { "time_ms": 1_700_003_600_000_u64, "price": 65_000.0 }, "c": { "time_ms": 1_700_007_200_000_u64, "price": 60_000.0 } } }
        ]));

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");
        let chart = &terminal.charts[&7];

        assert_eq!(result["success"], true);
        assert_eq!(result["operations"].as_array().map(Vec::len), Some(9));
        assert!(
            result["operations"]
                .as_array()
                .is_some_and(|rows| rows.iter().all(|row| row["outcome"] == "created"))
        );
        assert_eq!(chart.annotations.len(), 9);
        assert_annotations_mirrored(chart);
        assert_eq!(chart.next_annotation_id, 9);
        let rectangle = chart
            .annotations
            .iter()
            .find(|annotation| matches!(annotation.kind, AnnotationKind::Rectangle { .. }))
            .expect("rectangle");
        assert_eq!(rectangle.style.label.as_deref(), Some("Demand zone"));
        assert_eq!(rectangle.style.width, 2.5);
        assert_eq!(rectangle.style.line_style, LineStyle::Dashed);
        assert!(!rectangle.style.locked);
        assert!(rectangle.style.visible);
    }

    #[test]
    fn assistant_drawing_add_is_idempotent() {
        let mut terminal = terminal_with_running_drawing_tool();
        let payload = drawing_payload(serde_json::json!([{
            "operation": "add",
            "chart_id": 7,
            "drawing": { "type": "horizontal_level", "price": 60_000.0 }
        }]));

        let (first, _task) = terminal.handle_agent_host_action(&payload);
        let (second, _task) = terminal.handle_agent_host_action(&payload);
        let first: serde_json::Value = serde_json::from_str(&first).expect("first result");
        let second: serde_json::Value = serde_json::from_str(&second).expect("second result");

        assert_eq!(first["operations"][0]["outcome"], "created");
        assert_eq!(second["operations"][0]["outcome"], "already_present");
        assert_eq!(first["operations"][0]["drawing_id"], 0);
        assert_eq!(second["operations"][0]["drawing_id"], 0);
        assert_eq!(terminal.charts[&7].annotations.len(), 1);
        assert_eq!(terminal.charts[&7].next_annotation_id, 1);
    }

    #[test]
    fn failed_drawing_batch_is_atomic() {
        let mut terminal = terminal_with_running_drawing_tool();
        let locked = Annotation {
            id: 4,
            kind: AnnotationKind::HorizontalLevel { price: 55_000.0 },
            style: AnnotationStyle {
                locked: true,
                ..AnnotationStyle::default()
            },
        };
        {
            let chart = terminal.charts.get_mut(&7).expect("chart");
            chart.annotations.push(locked.clone());
            chart.chart.annotations = chart.annotations.clone();
            chart.next_annotation_id = 5;
        }
        let payload = drawing_payload(serde_json::json!([
            { "operation": "add", "chart_id": 7, "drawing": { "type": "horizontal_level", "price": 60_000.0 } },
            { "operation": "remove", "chart_id": 7, "drawing_id": 4 }
        ]));

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");
        let chart = &terminal.charts[&7];

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "drawing_locked");
        assert_eq!(chart.annotations.len(), 1);
        assert_eq!(chart.annotations[0].id, locked.id);
        assert_eq!(chart.annotations[0].kind, locked.kind);
        assert_eq!(chart.annotations[0].style, locked.style);
        assert_annotations_mirrored(chart);
        assert_eq!(chart.next_annotation_id, 5);
    }

    #[test]
    fn assistant_remove_clears_selection_and_canvas_copy() {
        let mut terminal = terminal_with_running_drawing_tool();
        let annotation = Annotation {
            id: 3,
            kind: AnnotationKind::VerticalLine {
                time: 1_700_000_000_000,
            },
            style: AnnotationStyle::default(),
        };
        {
            let chart = terminal.charts.get_mut(&7).expect("chart");
            chart.annotations.push(annotation.clone());
            chart.chart.annotations = chart.annotations.clone();
            chart.selected_annotation = Some(annotation.id);
            chart.next_annotation_id = 4;
        }
        let payload = drawing_payload(serde_json::json!([{
            "operation": "remove",
            "chart_id": 7,
            "drawing_id": 3
        }]));

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");
        let chart = &terminal.charts[&7];

        assert_eq!(result["success"], true);
        assert_eq!(result["operations"][0]["outcome"], "removed");
        assert!(chart.annotations.is_empty());
        assert!(chart.chart.annotations.is_empty());
        assert_eq!(chart.selected_annotation, None);
    }

    #[test]
    fn drawing_action_requires_the_matching_running_tool() {
        let mut terminal = terminal_with_running_tool();
        let payload = drawing_payload(serde_json::json!([{
            "operation": "add",
            "chart_id": 7,
            "drawing": { "type": "horizontal_level", "price": 60_000.0 }
        }]));

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["action"], "manage_chart_drawings");
        assert_eq!(result["error"]["code"], "inactive_tool_call");
        assert!(terminal.charts[&7].annotations.is_empty());
    }

    #[test]
    fn invalid_drawing_label_does_not_mutate_the_chart() {
        let mut terminal = terminal_with_running_drawing_tool();
        let payload = drawing_payload(serde_json::json!([{
            "operation": "add",
            "chart_id": 7,
            "drawing": {
                "type": "horizontal_level",
                "price": 60_000.0,
                "style": { "label": "unsafe\nlabel" }
            }
        }]));

        let (result, _task) = terminal.handle_agent_host_action(&payload);
        let result: serde_json::Value = serde_json::from_str(&result).expect("result");

        assert_eq!(result["success"], false);
        assert_eq!(result["error"]["code"], "invalid_drawing");
        assert!(terminal.charts[&7].annotations.is_empty());
    }
}
