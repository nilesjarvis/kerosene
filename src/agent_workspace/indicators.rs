use super::{AgentHostActionError, MAX_INDICATOR_CHANGES, MAX_TARGET_CHARTS};
use crate::app_state::TradingTerminal;
use crate::chart_indicator::ChartIndicatorId;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::Task;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Assistant Indicator Changes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChartIndicatorChange {
    indicator_id: ChartIndicatorId,
    enabled: bool,
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
    label: String,
    previous_enabled: bool,
    enabled: bool,
    outcome: &'static str,
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

impl TradingTerminal {
    pub(super) fn apply_agent_chart_indicator_changes(
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
                    label: change
                        .indicator_id
                        .moving_average_label(&instance.macro_indicators)
                        .unwrap_or_else(|| change.indicator_id.label().to_string()),
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

        let persistence_scheduled = self.persist_agent_workspace_changes(changed_any);
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
}

pub(super) fn failure_result(
    code: &'static str,
    message: impl Into<String>,
) -> (String, Task<Message>) {
    (
        AgentHostActionResponse::failure(code, message).into_json(),
        Task::none(),
    )
}
