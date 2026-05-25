use super::super::super::fees::user_fee_rates_from_value;
use crate::account::{
    AccountAbstractionMode, AccountDataCompleteness, AccountDataSection, FundingEntry,
    SpotClearinghouseState, UserFeeRates,
};

use serde_json::Value;

// ---------------------------------------------------------------------------
// Best-effort Bootstrap Responses
// ---------------------------------------------------------------------------

pub(in crate::account::data::bootstrap) fn fee_rates_from_best_effort_value(
    raw: Result<Value, String>,
    completeness: &mut AccountDataCompleteness,
) -> UserFeeRates {
    match raw {
        Ok(raw) => user_fee_rates_from_value(&raw),
        Err(error) => {
            completeness.mark_incomplete(AccountDataSection::Fees, error);
            UserFeeRates::default()
        }
    }
}

pub(in crate::account::data::bootstrap) fn record_best_effort_section_warnings(
    completeness: &mut AccountDataCompleteness,
    warnings: Vec<String>,
) {
    for warning in warnings {
        if warning.starts_with("frontendOpenOrders") {
            completeness.mark_incomplete(AccountDataSection::OpenOrders, warning);
        } else if warning.starts_with("userFills") {
            completeness.mark_incomplete(AccountDataSection::Fills, warning);
        } else {
            completeness.mark_incomplete(AccountDataSection::Positions, warning);
        }
    }
}

pub(in crate::account::data::bootstrap) fn account_abstraction_from_best_effort_value(
    raw: Result<Value, String>,
    spot: &SpotClearinghouseState,
    completeness: &mut AccountDataCompleteness,
) -> AccountAbstractionMode {
    if spot.portfolio_margin_enabled {
        return AccountAbstractionMode::PortfolioMargin;
    }

    match raw {
        Ok(raw) => raw
            .as_str()
            .map(AccountAbstractionMode::from_api_value)
            .unwrap_or_else(|| AccountAbstractionMode::Unknown(raw.to_string())),
        Err(error) => {
            completeness.mark_incomplete(AccountDataSection::Positions, error);
            AccountAbstractionMode::Unknown("unavailable".to_string())
        }
    }
}

pub(in crate::account::data::bootstrap) async fn funding_history_from_response(
    response: Result<reqwest::Response, reqwest::Error>,
    completeness: &mut AccountDataCompleteness,
) -> Vec<FundingEntry> {
    match response {
        Ok(response) if response.status().is_success() => match response.json().await {
            Ok(entries) => entries,
            Err(e) => {
                completeness.mark_incomplete(
                    AccountDataSection::Funding,
                    format!("userFunding parse failed: {e}"),
                );
                Vec::new()
            }
        },
        Ok(response) => {
            completeness.mark_incomplete(
                AccountDataSection::Funding,
                format!("userFunding request failed with HTTP {}", response.status()),
            );
            Vec::new()
        }
        Err(e) => {
            completeness.mark_incomplete(
                AccountDataSection::Funding,
                format!("userFunding request failed: {e}"),
            );
            Vec::new()
        }
    }
}

pub(in crate::account::data::bootstrap) async fn fee_rates_from_response(
    response: Result<reqwest::Response, reqwest::Error>,
    completeness: &mut AccountDataCompleteness,
) -> UserFeeRates {
    match response {
        Ok(response) => {
            if response.status().is_success() {
                fee_rates_from_best_effort_value(
                    response
                        .json::<Value>()
                        .await
                        .map_err(|e| format!("userFees parse failed: {e}")),
                    completeness,
                )
            } else {
                completeness.mark_incomplete(
                    AccountDataSection::Fees,
                    format!("userFees request failed with HTTP {}", response.status()),
                );
                UserFeeRates::default()
            }
        }
        Err(e) => {
            completeness.mark_incomplete(
                AccountDataSection::Fees,
                format!("userFees request failed: {e}"),
            );
            UserFeeRates::default()
        }
    }
}
