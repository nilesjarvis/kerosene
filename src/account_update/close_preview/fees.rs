use crate::api::proxy::HyperliquidRequestExt;
use serde_json::Value;

/// HIP-3 fee scales can change independently of the account's base taker rate.
/// Fetch them when opening the preview instead of assuming every DEX doubles it.
pub(super) async fn fetch_market_fee_multiplier(
    coin: String,
    quote_token: Option<u32>,
    stable_pair: bool,
) -> Result<f64, String> {
    let (base_multiplier, deployer_share, quote_token) =
        if let Some(dex) = crate::helpers::hip3_dex(&coin) {
            let raw = info(serde_json::json!({"type": "meta", "dex": dex})).await?;
            perp_fee_terms(&raw, &coin).ok_or("Market fee terms unavailable")?
        } else {
            (
                if stable_pair { 0.2 } else { 1.0 },
                0.0,
                quote_token.ok_or("Quote token unavailable")?,
            )
        };
    let aligned = if quote_token == 0 {
        false
    } else {
        // PnL is displayed in USD, so only dollar-stable quote assets qualify.
        let meta = info(serde_json::json!({"type": "spotMeta"})).await?;
        let stable = meta
            .get("tokens")
            .and_then(Value::as_array)
            .and_then(|tokens| {
                tokens.iter().find(|token| {
                    token.get("index").and_then(Value::as_u64) == Some(u64::from(quote_token))
                })
            })
            .and_then(|token| token.get("name"))
            .and_then(Value::as_str)
            .is_some_and(crate::helpers::is_usd_stable_fee_token);
        if !stable {
            return Err("Non-USD collateral estimate unavailable".into());
        }
        info(serde_json::json!({"type": "alignedQuoteTokenInfo", "token": quote_token}))
            .await?
            .get("isAligned")
            .and_then(Value::as_bool)
            .ok_or("Aligned quote fee terms unavailable")?
    };
    Ok(base_multiplier * aligned_taker_multiplier(deployer_share, aligned))
}

async fn info(payload: Value) -> Result<Value, String> {
    crate::api::CLIENT
        .post(crate::api::API_URL)
        .json(&payload)
        .send_info()
        .await
        .map_err(|_| "Market fee terms unavailable".to_string())?
        .json()
        .await
        .map_err(|_| "Market fee terms unavailable".to_string())
}

fn aligned_taker_multiplier(deployer_share: f64, aligned: bool) -> f64 {
    if aligned {
        (1.0 - deployer_share) * 0.8 + deployer_share
    } else {
        1.0
    }
}

fn perp_fee_terms(meta: &Value, coin: &str) -> Option<(f64, f64, u32)> {
    let quote_token = u32::try_from(meta.get("collateralToken")?.as_u64()?).ok()?;
    let asset = meta
        .get("universe")?
        .as_array()?
        .iter()
        .find(|asset| asset.get("name").and_then(Value::as_str) == Some(coin))?;
    let scale = asset
        .get("deployerFeeScale")?
        .as_str()?
        .parse::<f64>()
        .ok()?;
    if !scale.is_finite() || !(0.0..=3.0).contains(&scale) {
        return None;
    }
    let growth = match asset.get("growthMode") {
        None | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(Value::String(value)) if value == "enabled" => true,
        Some(Value::String(value)) if value == "disabled" => false,
        _ => return None,
    };
    let multiplier = if scale < 1.0 {
        1.0 + scale
    } else {
        2.0 * scale
    };
    let share = if scale < 1.0 {
        scale / (1.0 + scale)
    } else {
        0.5
    };
    Some((
        multiplier * if growth { 0.1 } else { 1.0 },
        share,
        quote_token,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hip3_fees_apply_growth_and_deployer_scale_and_require_quote_terms() {
        let mut meta = serde_json::json!({
            "collateralToken": 0,
            "universe": [{"name": "xyz:TEST", "deployerFeeScale": "1.0", "growthMode": "enabled"}]
        });
        assert_eq!(perp_fee_terms(&meta, "xyz:TEST"), Some((0.2, 0.5, 0)));
        meta["universe"][0]["growthMode"] = Value::Bool(false);
        meta["universe"][0]["deployerFeeScale"] = Value::String("0.5".into());
        assert_eq!(perp_fee_terms(&meta, "xyz:TEST"), Some((1.5, 1.0 / 3.0, 0)));
        meta["universe"][0]["deployerFeeScale"] = Value::String("3".into());
        assert_eq!(perp_fee_terms(&meta, "xyz:TEST"), Some((6.0, 0.5, 0)));
        assert_eq!(perp_fee_terms(&meta, "xyz:OTHER"), None);
        meta["collateralToken"] = Value::from(360);
        assert_eq!(perp_fee_terms(&meta, "xyz:TEST"), Some((6.0, 0.5, 360)));
        assert_eq!(aligned_taker_multiplier(0.5, true), 0.9);
        assert_eq!(aligned_taker_multiplier(0.0, true), 0.8);
        assert_eq!(aligned_taker_multiplier(0.5, false), 1.0);
    }

    #[test]
    fn malformed_market_fee_terms_do_not_fall_back_to_a_base_rate() {
        for scale in ["NaN", "inf", "-1", "4", ""] {
            let meta = serde_json::json!({
                "collateralToken": 0,
                "universe": [{"name": "xyz:TEST", "deployerFeeScale": scale}]
            });
            assert!(perp_fee_terms(&meta, "xyz:TEST").is_none());
        }
        let meta = serde_json::json!({
            "collateralToken": 0,
            "universe": [{"name": "xyz:TEST", "deployerFeeScale": "1", "growthMode": "unknown"}]
        });
        assert!(perp_fee_terms(&meta, "xyz:TEST").is_none());
        assert!(perp_fee_terms(&serde_json::json!({}), "xyz:TEST").is_none());
    }
}
