// ---------------------------------------------------------------------------
// Fill Fee Conversion
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

/// Dollar-stable tokens whose fees are already USD-denominated, mirroring the
/// quote-token classification in `account::spot_cost_basis`.
pub(crate) fn is_usd_stable_fee_token(fee_token: &str) -> bool {
    ["USDC", "USDE", "USDT0", "USDH"]
        .iter()
        .any(|stable| fee_token.eq_ignore_ascii_case(stable))
}

/// Convert a spot/outcome fill fee into USD.
///
/// Spot fees are charged in the token received: a *buy* pays its fee in the
/// base token (e.g. a `HYPE/USDC` buy is charged in HYPE), so it converts to
/// USD by multiplying by `px`, the pair's price per base unit in its
/// dollar-stable quote. A *sell* pays its fee in the quote token — USDC or
/// another dollar stable (USDT0/USDH/USDE) — which is already USD-denominated
/// and is returned unchanged, as is an empty fee token (perp fills).
pub(crate) fn non_perp_fee_usd(fee: f64, fee_token: &str, px: f64) -> f64 {
    if fee_token.is_empty() || is_usd_stable_fee_token(fee_token) {
        fee
    } else {
        fee * px
    }
}
