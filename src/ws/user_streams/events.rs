use crate::account::{
    AssetPosition, ClearinghouseState, OpenOrder, SpotBalance, UserFill, WalletPositionDetail,
    normalize_dex_asset_position_coins,
};
use crate::helpers::positive_finite_value;

use serde_json::Value;
use std::collections::HashMap;

use super::model::{KeyedUserData, WsUserData};
use super::routing::matching_user_payload_address;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// User Stream Event Parsing
// ---------------------------------------------------------------------------

pub(super) fn parse_user_stream_message(
    channel: &str,
    data: &Value,
    target_addr: Option<&str>,
    mids_addr: Option<String>,
) -> Option<KeyedUserData> {
    match channel {
        "allDexsClearinghouseState" => parse_all_dex_positions(data, target_addr),
        "openOrders" => parse_open_orders(data, target_addr),
        "userFills" => parse_user_fills(data, target_addr),
        "spotState" => parse_spot_balances(data, target_addr),
        "allMids" => parse_all_mids(data, mids_addr),
        _ => None,
    }
}

fn parse_all_dex_positions(data: &Value, target_addr: Option<&str>) -> Option<KeyedUserData> {
    let source_addr = matching_user_payload_address(data, target_addr)?;
    let states_arr = data.get("clearinghouseStates")?.as_array()?;

    let mut main_state: Option<ClearinghouseState> = None;
    let mut states_by_dex: HashMap<String, ClearinghouseState> = HashMap::new();
    let mut all_positions: Vec<AssetPosition> = Vec::new();
    let mut position_details: Vec<WalletPositionDetail> = Vec::new();

    for entry in states_arr {
        if let Some(arr) = entry.as_array()
            && arr.len() >= 2
        {
            let dex_name = arr[0].as_str().unwrap_or("");
            if let Ok(mut clearinghouse) =
                serde_json::from_value::<ClearinghouseState>(arr[1].clone())
            {
                normalize_dex_asset_position_coins(dex_name, &mut clearinghouse.asset_positions);
                all_positions.extend(clearinghouse.asset_positions.clone());
                position_details.extend(clearinghouse.asset_positions.iter().cloned().map(
                    |asset_position| WalletPositionDetail {
                        dex: dex_name.to_string(),
                        asset_position,
                    },
                ));
                states_by_dex.insert(dex_name.to_string(), clearinghouse.clone());
                if dex_name.is_empty() {
                    main_state = Some(clearinghouse);
                }
            }
        }
    }

    main_state.map(|state| {
        (
            Some(source_addr),
            WsUserData::AllDexPositions {
                main_state: Box::new(state),
                states_by_dex,
                all_positions,
                position_details,
            },
        )
    })
}

fn parse_open_orders(data: &Value, target_addr: Option<&str>) -> Option<KeyedUserData> {
    let source_addr = matching_user_payload_address(data, target_addr)?;
    let dex = data
        .get("dex")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let orders_val = data.get("orders")?;
    let orders = serde_json::from_value::<Vec<OpenOrder>>(orders_val.clone()).ok()?;
    Some((Some(source_addr), WsUserData::OpenOrders { dex, orders }))
}

fn parse_user_fills(data: &Value, target_addr: Option<&str>) -> Option<KeyedUserData> {
    let source_addr = matching_user_payload_address(data, target_addr)?;
    let is_snapshot = data
        .get("isSnapshot")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let fills_val = data.get("fills")?;
    let fills = serde_json::from_value::<Vec<UserFill>>(fills_val.clone()).ok()?;
    Some((Some(source_addr), WsUserData::Fills { fills, is_snapshot }))
}

fn parse_spot_balances(data: &Value, target_addr: Option<&str>) -> Option<KeyedUserData> {
    let source_addr = matching_user_payload_address(data, target_addr)?;
    let spot_state = data.get("spotState")?;
    let balances_val = spot_state.get("balances")?;
    let balances = serde_json::from_value::<Vec<SpotBalance>>(balances_val.clone()).ok()?;
    Some((Some(source_addr), WsUserData::SpotBalances(balances)))
}

fn parse_all_mids(data: &Value, source_addr: Option<String>) -> Option<KeyedUserData> {
    let mids_val = data.get("mids")?;
    let mids_str = serde_json::from_value::<HashMap<String, String>>(mids_val.clone()).ok()?;
    let mids = mids_str
        .into_iter()
        .filter_map(|(symbol, price)| {
            price
                .parse::<f64>()
                .ok()
                .and_then(positive_finite_value)
                .map(|mid| (symbol, mid))
        })
        .collect();
    Some((source_addr, WsUserData::AllMids(mids)))
}
