use super::{
    CapturedAgentKey, ExchangeResponse, actions::HyperliquidL1Action, client::post_order_action,
};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum TpslKind {
    #[serde(rename = "tp")]
    TakeProfit,
    #[serde(rename = "sl")]
    StopLoss,
}

impl TpslKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::TakeProfit => "Take profit",
            Self::StopLoss => "Stop loss",
        }
    }
}

#[derive(Clone)]
pub(crate) struct PositionTpslOrder {
    pub(crate) asset: u32,
    pub(crate) is_buy: bool,
    pub(crate) price: String,
    pub(crate) trigger_px: String,
    pub(crate) kind: TpslKind,
    pub(crate) cloid: String,
}

impl fmt::Debug for PositionTpslOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PositionTpslOrder")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

pub(crate) async fn place_position_tpsl(
    key: CapturedAgentKey,
    orders: Vec<PositionTpslOrder>,
) -> Result<ExchangeResponse, String> {
    post_order_action(key, HyperliquidL1Action::position_tpsl(orders)).await
}

#[cfg(test)]
mod tests;
