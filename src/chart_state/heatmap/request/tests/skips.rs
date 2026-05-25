use super::*;

#[test]
fn request_planner_skips_disabled_muted_fetching_or_unsupported_inputs() {
    let candles = vec![candle(1_000, 90.0, 110.0), candle(2_000, 95.0, 120.0)];

    let mut disabled = context(&candles, None);
    disabled.show_heatmap = false;
    assert!(optional_request_or_panic(disabled).is_none());

    let mut fetching = context(&candles, None);
    fetching.heatmap_fetching = true;
    assert!(optional_request_or_panic(fetching).is_none());

    let mut muted = context(&candles, None);
    muted.muted = true;
    assert!(optional_request_or_panic(muted).is_none());

    let mut unsupported = context(&candles, None);
    unsupported.coin = None;
    assert!(optional_request_or_panic(unsupported).is_none());

    let empty: Vec<Candle> = Vec::new();
    assert!(optional_request_or_panic(context(&empty, None)).is_none());
}
