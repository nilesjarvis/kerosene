use crate::session_data_state::SessionDataLookback;
use serde::{Deserialize, Serialize};

use super::super::default_symbol;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionDataConfig {
    #[serde(default)]
    pub id: u64,
    #[serde(default = "default_symbol")]
    pub symbol: String,
    #[serde(default)]
    pub lookback: SessionDataLookback,
}
