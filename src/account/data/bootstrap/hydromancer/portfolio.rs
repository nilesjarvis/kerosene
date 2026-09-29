use crate::account::{
    AccountAbstractionMode, AccountDataFetchScope, ClearinghouseState, HIP3_DEXES,
    SpotClearinghouseState, normalize_dex_asset_position_coins,
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, fmt};

pub(crate) type PortfolioClearinghouses = (
    ClearinghouseState,
    HashMap<String, ClearinghouseState>,
    Vec<ClearinghouseState>,
);

pub(crate) fn hydromancer_portfolio_chunk_size(scope: &AccountDataFetchScope) -> usize {
    match scope {
        AccountDataFetchScope::AllMarkets { .. } => 100,
        AccountDataFetchScope::Hip3Dex { .. } => 500,
    }
}

pub(crate) struct HydromancerPortfolioState {
    clearinghouse_state: Value,
    spot_clearinghouse_state: Value,
    user_abstraction: Value,
}

impl fmt::Debug for HydromancerPortfolioState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HydromancerPortfolioState")
            .field("clearinghouse_state", &"<redacted>")
            .field("spot_clearinghouse_state", &"<redacted>")
            .field("user_abstraction", &"<redacted>")
            .finish()
    }
}

impl HydromancerPortfolioState {
    /// Test-only constructor accepting the same wire shape as
    /// `parse_portfolio_state`, for exercising snapshot conversion without a
    /// network fetch.
    #[cfg(test)]
    pub(crate) fn from_raw_for_tests(raw: Value) -> Result<Self, String> {
        parse_portfolio_state(raw)
    }

    pub(crate) fn account_abstraction(&self) -> AccountAbstractionMode {
        self.user_abstraction
            .as_str()
            .map(AccountAbstractionMode::from_api_value)
            .unwrap_or_else(|| AccountAbstractionMode::Unknown(self.user_abstraction.to_string()))
    }

    pub(crate) fn spot_clearinghouse(&self) -> Result<SpotClearinghouseState, String> {
        SpotClearinghouseState::deserialize(&self.spot_clearinghouse_state)
            .map_err(|e| format!("spotClearinghouseState deserialize failed: {e}"))
    }

    pub(crate) fn clearinghouses_for_scope(
        &self,
        scope: &AccountDataFetchScope,
    ) -> Result<PortfolioClearinghouses, String> {
        if self.clearinghouse_state.get("marginSummary").is_some() {
            let clearinghouse = parse_clearinghouse_state("", &self.clearinghouse_state)?;
            let mut clearinghouses_by_dex = HashMap::new();
            clearinghouses_by_dex.insert(String::new(), clearinghouse.clone());
            return Ok((clearinghouse, clearinghouses_by_dex, Vec::new()));
        }

        let states = self
            .clearinghouse_state
            .as_object()
            .ok_or_else(|| "portfolioState clearinghouseState was not an object".to_string())?;
        let native_raw = states
            .get("native")
            .or_else(|| states.get(""))
            .ok_or_else(|| "portfolioState missing native clearinghouseState".to_string())?;
        let native = parse_clearinghouse_state("", native_raw)?;
        let mut clearinghouses_by_dex = HashMap::new();
        clearinghouses_by_dex.insert(String::new(), native.clone());

        let mut hip3_states = Vec::new();
        for dex in scope.hip3_dexes(HIP3_DEXES) {
            let Some(raw) = states.get(dex) else {
                continue;
            };
            let state = parse_clearinghouse_state(dex, raw)?;
            clearinghouses_by_dex.insert(dex.to_string(), state.clone());
            hip3_states.push(state);
        }

        Ok((native, clearinghouses_by_dex, hip3_states))
    }
}

pub(super) fn parse_portfolio_state(mut raw: Value) -> Result<HydromancerPortfolioState, String> {
    Ok(HydromancerPortfolioState {
        clearinghouse_state: raw
            .get_mut("clearinghouseState")
            .map(Value::take)
            .ok_or_else(|| "portfolioState missing clearinghouseState".to_string())?,
        spot_clearinghouse_state: raw
            .get_mut("spotClearinghouseState")
            .map(Value::take)
            .ok_or_else(|| "portfolioState missing spotClearinghouseState".to_string())?,
        user_abstraction: raw
            .get_mut("userAbstraction")
            .map(Value::take)
            .unwrap_or_else(|| Value::String("default".to_string())),
    })
}

pub(super) fn merge_native_and_dex_portfolio_states(
    native_raw: Value,
    dex_raw: Value,
    dex: &str,
) -> Result<HydromancerPortfolioState, String> {
    let native = parse_portfolio_state(native_raw)?;
    let dex_state = parse_portfolio_state(dex_raw)?;
    let mut clearinghouse_state = serde_json::Map::new();
    clearinghouse_state.insert("native".to_string(), native.clearinghouse_state);
    clearinghouse_state.insert(dex.to_string(), dex_state.clearinghouse_state);
    Ok(HydromancerPortfolioState {
        clearinghouse_state: Value::Object(clearinghouse_state),
        spot_clearinghouse_state: native.spot_clearinghouse_state,
        user_abstraction: native.user_abstraction,
    })
}

fn parse_clearinghouse_state(dex: &str, raw: &Value) -> Result<ClearinghouseState, String> {
    let mut clearinghouse = ClearinghouseState::deserialize(raw)
        .map_err(|e| format!("{dex} clearinghouseState deserialize failed: {e}"))?;
    normalize_dex_asset_position_coins(dex, &mut clearinghouse.asset_positions);
    Ok(clearinghouse)
}

pub(super) struct HydromancerBatchPortfolioStates {
    pub(super) successful_states: Vec<(String, Value)>,
    pub(super) failed_wallets: Vec<String>,
}

pub(super) fn parse_batch_portfolio_states(
    mut raw: Value,
) -> Result<HydromancerBatchPortfolioStates, String> {
    let Some(Value::Array(successful_raw)) =
        take_aliased_field(&mut raw, "successful_states", "successfulStates")
    else {
        return Err("batchPortfolioStates missing successful_states".to_string());
    };
    let mut successful_states = Vec::new();
    for item in successful_raw {
        let Value::Array(pair) = item else {
            return Err("batchPortfolioStates successful state was not a tuple".to_string());
        };
        let [address, state]: [Value; 2] = pair.try_into().map_err(|_| {
            "batchPortfolioStates successful state tuple had wrong length".to_string()
        })?;
        let Value::String(address) = address else {
            return Err("batchPortfolioStates successful state missing address".to_string());
        };
        successful_states.push((address, state));
    }

    let failed_wallets = match take_aliased_field(&mut raw, "failed_wallets", "failedWallets") {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|item| match item {
                Value::String(address) => Some(address),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };

    Ok(HydromancerBatchPortfolioStates {
        successful_states,
        failed_wallets,
    })
}

fn take_aliased_field(raw: &mut Value, name: &str, alias: &str) -> Option<Value> {
    raw.get_mut(name)
        .map(Value::take)
        .or_else(|| raw.get_mut(alias).map(Value::take))
}

#[cfg(test)]
mod tests;
