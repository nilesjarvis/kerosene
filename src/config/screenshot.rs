use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ChartScreenshotSettingsConfig {
    #[serde(default)]
    pub obscure_position_entry: bool,
    #[serde(default)]
    pub hide_positions_and_orders: bool,
}
