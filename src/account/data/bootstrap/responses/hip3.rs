use crate::account::{
    AccountDataCompleteness, AccountDataSection, ClearinghouseState, OpenOrder,
    normalize_dex_asset_position_coins, normalize_dex_open_order_coins,
};

use serde_json::Value;

// ---------------------------------------------------------------------------
// HIP-3 Bootstrap Responses
// ---------------------------------------------------------------------------

pub(in crate::account::data::bootstrap) async fn hip3_clearinghouse_from_response(
    dex: &str,
    response: Result<reqwest::Response, reqwest::Error>,
    completeness: &mut AccountDataCompleteness,
) -> Option<ClearinghouseState> {
    match response {
        Ok(response) if response.status().is_success() => match response.json::<Value>().await {
            Ok(raw) => match serde_json::from_value::<ClearinghouseState>(raw) {
                Ok(mut clearinghouse) => {
                    normalize_dex_asset_position_coins(dex, &mut clearinghouse.asset_positions);
                    Some(clearinghouse)
                }
                Err(e) => {
                    completeness.mark_incomplete(
                        AccountDataSection::Positions,
                        format!("HIP-3 clearinghouseState parse failed: {e}"),
                    );
                    None
                }
            },
            Err(e) => {
                completeness.mark_incomplete(
                    AccountDataSection::Positions,
                    format!("HIP-3 clearinghouseState response parse failed: {e}"),
                );
                None
            }
        },
        Ok(response) => {
            completeness.mark_incomplete(
                AccountDataSection::Positions,
                format!(
                    "HIP-3 clearinghouseState request failed with HTTP {}",
                    response.status()
                ),
            );
            None
        }
        Err(e) => {
            completeness.mark_incomplete(
                AccountDataSection::Positions,
                format!("HIP-3 clearinghouseState request failed: {e}"),
            );
            None
        }
    }
}

pub(in crate::account::data::bootstrap) async fn hip3_open_orders_from_response(
    dex: &str,
    response: Result<reqwest::Response, reqwest::Error>,
    completeness: &mut AccountDataCompleteness,
) -> Option<Vec<OpenOrder>> {
    match response {
        Ok(response) if response.status().is_success() => {
            match response.json::<Vec<OpenOrder>>().await {
                Ok(mut orders) => {
                    normalize_dex_open_order_coins(dex, &mut orders);
                    Some(orders)
                }
                Err(e) => {
                    completeness.mark_incomplete(
                        AccountDataSection::OpenOrders,
                        format!("HIP-3 frontendOpenOrders parse failed: {e}"),
                    );
                    None
                }
            }
        }
        Ok(response) => {
            completeness.mark_incomplete(
                AccountDataSection::OpenOrders,
                format!(
                    "HIP-3 frontendOpenOrders request failed with HTTP {}",
                    response.status()
                ),
            );
            None
        }
        Err(e) => {
            completeness.mark_incomplete(
                AccountDataSection::OpenOrders,
                format!("HIP-3 frontendOpenOrders request failed: {e}"),
            );
            None
        }
    }
}
