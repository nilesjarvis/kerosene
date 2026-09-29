use crate::denomination::DisplayDenominationContext;
use crate::helpers::ellipsized_text;
use crate::hype_unstaking_state::{HYPE_CORE_WEI_PER_TOKEN, format_hype_wei};
use crate::wallet_state::address_book::WalletDisplay;

// ---------------------------------------------------------------------------
// Row Labels
// ---------------------------------------------------------------------------

pub(super) fn format_hype_amount_with_notional(
    amount_wei: u64,
    hype_mid: Option<f64>,
    denomination: &DisplayDenominationContext,
) -> String {
    let amount = format_hype_wei(amount_wei as u128);
    let notional = hype_mid
        .and_then(|mid| {
            let usd_value = amount_wei as f64 / HYPE_CORE_WEI_PER_TOKEN as f64 * mid;
            usd_value
                .is_finite()
                .then(|| denomination.format_value(usd_value, 2))
        })
        .unwrap_or_else(|| "n/a".to_string());

    format!("{amount} ({notional})")
}

pub(super) fn hype_unstaking_wallet_label(display: &WalletDisplay) -> String {
    ellipsized_text(&display.primary, 26)
}

pub(super) fn hype_unstaking_wallet_tooltip(display: &WalletDisplay, address: &str) -> String {
    if display.has_label {
        format!("{} ({address})", display.primary)
    } else {
        format!("Copy {address}")
    }
}

pub(super) fn format_local_time_ms(time_ms: u64) -> String {
    let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(time_ms as i64) else {
        return "-".to_string();
    };
    dt.with_timezone(&chrono::Local)
        .format("%m/%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests;
