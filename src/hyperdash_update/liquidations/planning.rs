use crate::helpers::{parse_positive_finite_number, positive_finite_value};

// ---------------------------------------------------------------------------
// Liquidation Request Planning
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub(in crate::hyperdash_update::liquidations) enum LiquidationRequestPlan {
    Fetch { coin: String, mark: f64 },
    Status(String, bool),
    Wait,
}

pub(in crate::hyperdash_update::liquidations) struct LiquidationPlanContext<'a> {
    pub(in crate::hyperdash_update::liquidations) show_liquidations: bool,
    pub(in crate::hyperdash_update::liquidations) liquidation_fetching: bool,
    pub(in crate::hyperdash_update::liquidations) hyperdash_key_missing: bool,
    pub(in crate::hyperdash_update::liquidations) symbol: &'a str,
    pub(in crate::hyperdash_update::liquidations) ticker_muted: bool,
    pub(in crate::hyperdash_update::liquidations) coin: Option<&'a str>,
    pub(in crate::hyperdash_update::liquidations) mark: Option<f64>,
}

pub(in crate::hyperdash_update::liquidations) fn liquidation_request_plan(
    ctx: LiquidationPlanContext<'_>,
) -> LiquidationRequestPlan {
    if !ctx.show_liquidations || ctx.liquidation_fetching {
        return LiquidationRequestPlan::Wait;
    }
    if ctx.hyperdash_key_missing {
        return LiquidationRequestPlan::Status(
            "Add HyperDash key in Settings > Integrations".to_string(),
            true,
        );
    }
    if ctx.symbol.is_empty() || ctx.ticker_muted {
        return LiquidationRequestPlan::Wait;
    }
    let Some(coin) = ctx.coin else {
        return LiquidationRequestPlan::Status(
            "Liquidation overlay is only available for perp symbols".to_string(),
            true,
        );
    };
    let Some(mark) = ctx.mark else {
        return LiquidationRequestPlan::Status("LIQ waiting for mark price".to_string(), false);
    };

    LiquidationRequestPlan::Fetch {
        coin: coin.to_string(),
        mark,
    }
}

pub(in crate::hyperdash_update::liquidations) fn liquidation_mark_from_ctx(
    mark_px: Option<&str>,
    fallback_close: Option<f64>,
) -> Option<f64> {
    mark_px
        .and_then(parse_positive_finite_str)
        .or_else(|| fallback_close.and_then(positive_finite_value))
}

fn parse_positive_finite_str(value: &str) -> Option<f64> {
    parse_positive_finite_number(value)
}

pub(in crate::hyperdash_update::liquidations) fn liquidation_request_key(
    coin: &str,
    min: f64,
    max: f64,
    timestamp_secs: u64,
) -> String {
    format!("{coin}:{min:.8}:{max:.8}:{timestamp_secs}")
}

pub(in crate::hyperdash_update::liquidations) fn liquidation_request_coin(
    request_key: &str,
) -> &str {
    request_key.split_once(':').map_or("", |(coin, _)| coin)
}
