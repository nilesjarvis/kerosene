use super::quantity::{order_percentage_for_quantity, quantity_for_percentage};

mod position;

pub(in crate::order_update::form) use position::position_size_for_symbol;
use position::{percentage_for_position_quantity, position_quantity_for_percentage};

// ---------------------------------------------------------------------------
// Order Sizing Basis
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) enum OrderSizingBasis {
    MarginNotional { max_notional: f64 },
    ReduceOnlyPosition { position_size_coin: f64 },
}

impl OrderSizingBasis {
    pub(super) fn percentage_for_quantity(
        self,
        quantity: f64,
        quantity_is_usd: bool,
        reference_price: Option<f64>,
    ) -> f32 {
        match self {
            Self::MarginNotional { max_notional } => order_percentage_for_quantity(
                quantity,
                quantity_is_usd,
                reference_price,
                max_notional,
            ),
            Self::ReduceOnlyPosition { position_size_coin } => percentage_for_position_quantity(
                quantity,
                position_size_coin,
                quantity_is_usd,
                reference_price,
            ),
        }
    }

    pub(super) fn quantity_for_percentage(
        self,
        percentage: f32,
        quantity_is_usd: bool,
        reference_price: Option<f64>,
        decimals: usize,
    ) -> String {
        match self {
            Self::MarginNotional { max_notional } => quantity_for_percentage(
                percentage,
                max_notional,
                quantity_is_usd,
                reference_price,
                decimals,
            ),
            Self::ReduceOnlyPosition { position_size_coin } => position_quantity_for_percentage(
                percentage,
                position_size_coin,
                quantity_is_usd,
                reference_price,
                decimals,
            ),
        }
    }
}
