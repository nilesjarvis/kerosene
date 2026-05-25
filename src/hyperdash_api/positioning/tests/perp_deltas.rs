use super::*;

#[test]
fn perp_deltas_parse_response() {
    let parsed = perp_deltas_or_panic(
        r#"{
          "data": {
            "perpDeltas": {
              "market": "HYPE",
              "timeframe": "15m",
              "deltas": [{
                "address": "0xabc0000000000000000000000000000000000000",
                "current": -25.5,
                "delta": 10.25
              }]
            }
          }
        }"#,
    );

    assert_eq!(parsed.market, "HYPE");
    assert_eq!(parsed.timeframe, "15m");
    assert_eq!(parsed.deltas.len(), 1);
    assert_eq!(parsed.deltas[0].delta, 10.25);
}

#[test]
fn perp_deltas_truncates_large_result_set() {
    let mut deltas = String::new();
    for index in 0..(PERP_DELTAS_ENTRY_LIMIT + 3) {
        if index > 0 {
            deltas.push(',');
        }
        deltas.push_str(&format!(
            r#"{{"address":"0x{index:040x}","current":1.0,"delta":2.0}}"#
        ));
    }

    let payload = format!(
        concat!(
            r#"{{"data":{{"perpDeltas":{{"#,
            r#""market":"HYPE","timeframe":"15m","deltas":[{}]"#,
            r#"}}}}}}"#,
        ),
        deltas
    );

    let parsed = perp_deltas_or_panic(&payload);

    assert_eq!(parsed.deltas.len(), PERP_DELTAS_ENTRY_LIMIT);
}

#[test]
fn perp_deltas_response_chunk_cap_rejects_oversized_body_before_append() {
    let mut body = vec![b'a'; PERP_DELTAS_RESPONSE_MAX_BYTES - 1];
    let err = chunk_error_or_panic(&mut body, b"bb");

    assert_eq!(body.len(), PERP_DELTAS_RESPONSE_MAX_BYTES - 1);
    assert!(err.contains("HyperDash perp deltas response too large"));
    assert!(err.contains(&PERP_DELTAS_RESPONSE_MAX_BYTES.to_string()));
}

#[test]
fn perp_deltas_response_chunk_cap_accepts_exact_limit() {
    let mut body = vec![b'a'; PERP_DELTAS_RESPONSE_MAX_BYTES - 1];

    append_chunk_or_panic(&mut body, b"b");

    assert_eq!(body.len(), PERP_DELTAS_RESPONSE_MAX_BYTES);
}

#[test]
fn perp_deltas_reports_graphql_errors_without_data() {
    let error =
        perp_deltas_error_or_panic(r#"{"errors":[{"message":"invalid api key"}],"data":null}"#);

    assert!(error.contains("authentication failed"));
}
