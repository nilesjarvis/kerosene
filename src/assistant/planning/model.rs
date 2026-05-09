use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct AgentPlan {
    pub(super) objective: String,
    pub(super) symbols: Vec<String>,
    pub(super) interval: String,
    pub(super) lookback_days: Option<u32>,
    pub(super) strategy: String,
    pub(super) tranche_usd: Option<f64>,
    pub(super) drawdown_pct: Option<f64>,
    pub(super) assumptions: Vec<String>,
    pub(super) steps: Vec<String>,
    pub(super) dex: Option<String>,
}
