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
        main_state: Box<ClearinghouseState>,
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
}
