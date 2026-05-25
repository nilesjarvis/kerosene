use super::{normalize_dex_open_order_coins, open_order};

#[test]
fn hip3_open_order_stream_symbols_are_normalized() {
    let mut orders = vec![open_order(42, Some(false)), open_order(43, Some(false))];
    orders[1].coin = "flx:ETH".to_string();

    normalize_dex_open_order_coins("flx", &mut orders);

    assert_eq!(orders[0].coin, "flx:BTC");
    assert_eq!(orders[1].coin, "flx:ETH");
}

#[test]
fn main_dex_open_order_stream_symbols_stay_unprefixed() {
    let mut orders = vec![open_order(42, Some(false))];

    normalize_dex_open_order_coins("", &mut orders);

    assert_eq!(orders[0].coin, "BTC");
}
