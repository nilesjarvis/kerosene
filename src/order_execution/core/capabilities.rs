use super::{OrderOperation, OrderSurface};
use crate::api::MarketType;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Surface Capabilities and Labels
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderCapabilityError {
    UnsupportedMarketType {
        surface: OrderSurface,
        operation: OrderOperation,
        market_type: MarketType,
    },
}

impl OrderCapabilityError {
    pub(crate) fn status_text(self) -> String {
        match self {
            Self::UnsupportedMarketType {
                surface,
                operation,
                market_type,
            } => match market_type {
                MarketType::Outcome => format!(
                    "Outcome {} is not available from this control; use the main order ticket",
                    surface.outcome_action_label(operation)
                ),
                MarketType::Perp | MarketType::Spot => format!(
                    "{} {} does not support {} markets",
                    surface.label(),
                    operation.label(),
                    market_type_label(market_type)
                ),
            },
        }
    }
}

pub(crate) fn validate_surface_market_type(
    surface: OrderSurface,
    operation: OrderOperation,
    market_type: MarketType,
) -> Result<(), OrderCapabilityError> {
    if surface.allows_market_type(operation, market_type) {
        Ok(())
    } else {
        Err(OrderCapabilityError::UnsupportedMarketType {
            surface,
            operation,
            market_type,
        })
    }
}

impl OrderSurface {
    pub(crate) fn allows_market_type(
        self,
        operation: OrderOperation,
        market_type: MarketType,
    ) -> bool {
        match operation {
            OrderOperation::Cancel => true,
            OrderOperation::Modify => {
                matches!(self, Self::Move) || market_type != MarketType::Outcome
            }
            OrderOperation::UpdateLeverage => market_type == MarketType::Perp,
            OrderOperation::Place => match self {
                Self::Ticket | Self::Preset => true,
                Self::QuickOrder
                | Self::QuickTrade
                | Self::Hud
                | Self::ClosePosition
                | Self::Cluster
                | Self::Chase
                | Self::Twap => market_type != MarketType::Outcome,
                Self::ClusterClose | Self::Nuke => market_type == MarketType::Perp,
                Self::Move | Self::Cancel => true,
            },
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ticket => "Ticket",
            Self::Preset => "Preset",
            Self::QuickOrder => "Quick order",
            Self::QuickTrade => "Quick Trade order",
            Self::Hud => "HUD order",
            Self::ClosePosition => "Position close",
            Self::Cluster => "Wallet cluster",
            Self::ClusterClose => "Cluster close",
            Self::Nuke => "NUKE",
            Self::Chase => "Chase",
            Self::Twap => "TWAP",
            Self::Move => "Move order",
            Self::Cancel => "Cancel order",
        }
    }

    fn outcome_action_label(self, operation: OrderOperation) -> &'static str {
        match (self, operation) {
            (Self::QuickOrder, OrderOperation::Place) => "trading",
            (Self::QuickTrade, OrderOperation::Place) => "Quick Trade trading",
            (Self::Hud, OrderOperation::Place) => "HUD trading",
            (Self::ClosePosition, OrderOperation::Place) => "position closing",
            (Self::ClusterClose, OrderOperation::Place) => "cluster position closing",
            (Self::Chase, OrderOperation::Place) => "chase trading",
            (Self::Twap, OrderOperation::Place) => "TWAP trading",
            _ => operation.label(),
        }
    }

    pub(crate) fn orderability_context_label(self) -> &'static str {
        match self {
            Self::Ticket | Self::Preset | Self::Chase | Self::Twap => "Active",
            Self::Cluster => "Cluster",
            Self::QuickOrder | Self::QuickTrade | Self::Hud => "Chart",
            Self::ClosePosition | Self::ClusterClose | Self::Nuke => "Position",
            Self::Move | Self::Cancel => "Order",
        }
    }

    pub(super) fn uses_connected_account_state(self) -> bool {
        matches!(
            self,
            Self::Ticket
                | Self::Preset
                | Self::QuickOrder
                | Self::QuickTrade
                | Self::Hud
                | Self::ClosePosition
                | Self::Nuke
                | Self::Chase
                | Self::Twap
        )
    }

    pub(super) fn symbol_not_found_status_text(self, symbol_key: &str) -> String {
        match self {
            Self::QuickOrder
            | Self::QuickTrade
            | Self::Hud
            | Self::ClosePosition
            | Self::Cluster
            | Self::ClusterClose
            | Self::Nuke
            | Self::Move
            | Self::Cancel => {
                format!("Symbol '{symbol_key}' not found")
            }
            Self::Ticket | Self::Preset | Self::Chase | Self::Twap => {
                format!("Symbol '{symbol_key}' not found in exchange metadata")
            }
        }
    }
}

impl OrderOperation {
    fn label(self) -> &'static str {
        match self {
            Self::Place => "placement",
            Self::Cancel => "cancellation",
            Self::Modify => "modification",
            Self::UpdateLeverage => "leverage update",
        }
    }
}

fn market_type_label(market_type: MarketType) -> &'static str {
    match market_type {
        MarketType::Perp => "perpetual",
        MarketType::Spot => "spot",
        MarketType::Outcome => "outcome",
    }
}
