use crate::account::{
    AssetPosition, ClearinghouseState, OpenOrder, SpotBalance, UserFill, WalletPositionDetail,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// User Stream Models
// ---------------------------------------------------------------------------

pub type KeyedUserData = (Option<String>, WsUserData);

#[derive(Debug, Clone)]
pub enum WsUserData {
    AllDexPositions {
        main_state: Option<Box<ClearinghouseState>>,
        states_by_dex: HashMap<String, ClearinghouseState>,
        all_positions: Vec<AssetPosition>,
        position_details: Vec<WalletPositionDetail>,
    },
    OpenOrders {
        dex: String,
        orders: Vec<OpenOrder>,
    },
    Fills {
        fills: Vec<UserFill>,
        is_snapshot: bool,
    },
    SpotBalances(Vec<SpotBalance>),
    AllMids(HashMap<String, f64>),
    /// The broadcast fanout for the user-data WebSocket signalled
    /// `RecvError::Lagged`: at least `skipped` order/fill/position
    /// updates were dropped before this consumer could observe them. The
    /// downstream handler must treat local account state as stale and
    /// force a full `fetch_account_data` rather than continuing from an
    /// unknown state.
    Lagged {
        skipped: u64,
    },
}
