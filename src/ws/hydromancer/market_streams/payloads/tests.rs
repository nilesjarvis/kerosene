use super::*;
use crate::account::AssetContext;
use crate::api::{Candle, parse_ws_book};

fn assert_borrowed_items(select: fn(&Value) -> &[Value], frame: Value, expected_paths: &[&str]) {
    let items = select(&frame);
    assert_eq!(items.len(), expected_paths.len(), "frame: {frame}");
    for (item, path) in items.iter().zip(expected_paths) {
        assert!(
            std::ptr::eq(item, frame.pointer(path).expect("expected item exists")),
            "item should borrow {path} from frame: {frame}"
        );
    }
}

#[test]
fn selectors_borrow_direct_wrapped_and_batched_items_with_original_precedence() {
    for (select, item) in [
        (
            l2_book_items as fn(&Value) -> &[Value],
            serde_json::json!({"coin": "BTC", "levels": []}),
        ),
        (
            active_asset_ctx_items,
            serde_json::json!({"coin": "ETH", "ctx": {}}),
        ),
        (candle_items, serde_json::json!({"s": "BTC", "i": "1m"})),
    ] {
        assert_borrowed_items(select, item.clone(), &[""]);
        assert_borrowed_items(select, serde_json::json!({"data": item}), &["/data"]);
        assert_borrowed_items(
            select,
            serde_json::json!({"data": [null, item, false, {"unrelated": true}]}),
            &["/data/0", "/data/1", "/data/2", "/data/3"],
        );

        let mut direct_and_wrapped = item.clone();
        direct_and_wrapped["data"] = serde_json::json!([item]);
        assert_borrowed_items(select, direct_and_wrapped, &[""]);

        // Selection checks field presence; parsing and validation happen later.
        let mut null_fields = item.clone();
        for value in null_fields
            .as_object_mut()
            .expect("item object")
            .values_mut()
        {
            *value = Value::Null;
        }
        assert_borrowed_items(select, null_fields, &[""]);

        for frame in [
            Value::Null,
            serde_json::json!(7),
            serde_json::json!([item]),
            serde_json::json!({"data": []}),
            serde_json::json!({"data": null}),
            serde_json::json!({"data": {"data": item}}),
        ] {
            assert_borrowed_items(select, frame, &[]);
        }
    }
}

#[test]
fn book_and_candle_fallback_shapes_preserve_precedence() {
    assert_borrowed_items(
        l2_book_items,
        serde_json::json!({"data": {"coin": "incomplete"}, "books": [{"coin": "BTC"}, null]}),
        &["/books/0", "/books/1"],
    );
    assert_borrowed_items(
        l2_book_items,
        serde_json::json!({"books": {"coin": "BTC", "levels": []}}),
        &[],
    );
    assert_borrowed_items(
        l2_book_items,
        serde_json::json!({"data": [], "books": [{"coin": "BTC"}]}),
        &[],
    );
    assert_borrowed_items(
        l2_book_items,
        serde_json::json!({"data": {"coin": "BTC", "levels": []}, "books": [null]}),
        &["/data"],
    );

    // The legacy singular candle wrapper is selected even if malformed.
    for candle in [
        Value::Null,
        serde_json::json!([{"s": "BTC", "i": "1m"}]),
        serde_json::json!({"s": "BTC", "i": "1m"}),
    ] {
        assert_borrowed_items(
            candle_items,
            serde_json::json!({"data": {"s": "incomplete"}, "candle": candle}),
            &["/candle"],
        );
    }
    assert_borrowed_items(
        candle_items,
        serde_json::json!({"data": [], "candle": {"s": "BTC", "i": "1m"}}),
        &[],
    );
    assert_borrowed_items(
        candle_items,
        serde_json::json!({"data": {"s": "BTC", "i": "1m"}, "candle": null}),
        &["/data"],
    );
    assert_borrowed_items(
        active_asset_ctx_items,
        serde_json::json!({"books": [{"coin": "BTC", "ctx": {}}], "candle": {"coin": "BTC", "ctx": {}}}),
        &[],
    );
}

#[test]
fn l2_book_items_accept_single_object_and_data_array_batches() {
    let single = serde_json::json!({
        "channel": "l2Book",
        "data": {
            "coin": "BTC",
            "levels": [
                [{ "px": "100", "sz": "1" }],
                [{ "px": "101", "sz": "2" }]
            ]
        }
    });
    let single_items = l2_book_items(&single);
    assert_eq!(single_items.len(), 1);
    assert_eq!(
        single_items[0].get("coin").and_then(Value::as_str),
        Some("BTC")
    );
    assert!(parse_ws_book(&single_items[0]).is_some());

    let batch = serde_json::json!({
        "type": "l2Book",
        "data": [
            {
                "coin": "BTC",
                "levels": [
                    [{ "px": "100", "sz": "1" }],
                    [{ "px": "101", "sz": "2" }]
                ]
            },
            {
                "coin": "ETH",
                "levels": [
                    [{ "px": "10", "sz": "3" }],
                    [{ "px": "11", "sz": "4" }]
                ]
            }
        ]
    });
    let batch_items = l2_book_items(&batch);
    assert_eq!(batch_items.len(), 2);
    assert_eq!(
        batch_items[1].get("coin").and_then(Value::as_str),
        Some("ETH")
    );
}

#[test]
fn active_asset_ctx_items_accept_data_objects() {
    let value = serde_json::json!({
        "channel": "activeAssetCtx",
        "data": {
            "coin": "ETH",
            "ctx": {
                "oraclePx": "3230.1",
                "markPx": "3227.4",
                "midPx": "3228.25"
            }
        }
    });

    let items = active_asset_ctx_items(&value);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].get("coin").and_then(Value::as_str), Some("ETH"));
    let ctx = items[0].get("ctx").expect("ctx exists").clone();
    let ctx = serde_json::from_value::<AssetContext>(ctx).expect("ctx parses");
    assert_eq!(ctx.mid_px.as_deref(), Some("3228.25"));
}

#[test]
fn candle_items_accept_data_array_batches() {
    let value = serde_json::json!({
        "channel": "candle",
        "data": [
            {
                "s": "BTC",
                "i": "1m",
                "t": 10,
                "T": 69,
                "o": "1",
                "h": "2",
                "l": "0.5",
                "c": "1.5",
                "v": "12"
            }
        ]
    });

    let items = candle_items(&value);
    assert_eq!(items.len(), 1);
    let candle = serde_json::from_value::<Candle>(items[0].clone()).expect("candle parses");
    assert_eq!(candle.open_time, 10);
    assert_eq!(candle.close, 1.5);
}
