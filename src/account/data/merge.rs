use super::super::{ClearinghouseState, OpenOrder};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Bootstrap Data Merging
// ---------------------------------------------------------------------------

pub(super) fn merge_hip3_positions(
    mut clearinghouse: ClearinghouseState,
    hip3_states: Vec<ClearinghouseState>,
) -> ClearinghouseState {
    for state in hip3_states {
        clearinghouse.asset_positions.extend(state.asset_positions);
    }
    clearinghouse
}

pub(super) fn merge_hip3_open_orders(
    mut open_orders: Vec<OpenOrder>,
    hip3_orders: Vec<Vec<OpenOrder>>,
) -> Vec<OpenOrder> {
    for orders in hip3_orders {
        open_orders.extend(orders);
    }
    open_orders
}
