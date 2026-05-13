use self::parsing::{parse_reserve_states, parse_spot_token_names};
use self::snapshot::build_income_snapshot;
use super::http::{optional_response_value, response_json};
use super::model::{BorrowLendInterestEntry, BorrowLendUserState, IncomeSnapshot};
use crate::api::{API_URL, CLIENT};

use serde_json::Value;
use std::collections::HashMap;

mod parsing;
mod snapshot;

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

/// Fetch borrow/lend income data for a portfolio-margin account.
pub async fn fetch_income_data(address: String) -> Result<IncomeSnapshot, String> {
    fetch_income_data_from_url(CLIENT.clone(), API_URL, address).await
}

async fn fetch_income_data_from_url(
    client: reqwest::Client,
    url: &str,
    address: String,
) -> Result<IncomeSnapshot, String> {
    let reserve_fut = client
        .post(url)
        .json(&serde_json::json!({"type": "allBorrowLendReserveStates"}))
        .send();
    let user_state_fut = client
        .post(url)
        .json(&serde_json::json!({"type": "borrowLendUserState", "user": address}))
        .send();
    let interest_fut = client
        .post(url)
        .json(&serde_json::json!({"type": "userBorrowLendInterest", "user": address}))
        .send();
    let spot_meta_fut = client
        .post(url)
        .json(&serde_json::json!({"type": "spotMeta"}))
        .send();

    let (reserve_resp, user_state_resp, interest_resp, spot_meta_resp) =
        futures::future::join4(reserve_fut, user_state_fut, interest_fut, spot_meta_fut).await;

    let reserve_raw: Value = response_json(
        "allBorrowLendReserveStates",
        reserve_resp.map_err(|e| format!("allBorrowLendReserveStates request failed: {e}"))?,
    )
    .await?;

    if let Some(err) = reserve_raw.get("error").and_then(|v| v.as_str()) {
        return Err(format!("allBorrowLendReserveStates error: {err}"));
    }

    let reserve_by_token = parse_reserve_states(&reserve_raw);
    if reserve_by_token.is_empty() {
        let preview = reserve_raw.to_string();
        let snippet = if preview.len() > 180 {
            format!("{}...", &preview[..180])
        } else {
            preview
        };
        return Err(format!(
            "allBorrowLendReserveStates response had no parseable reserve entries: {snippet}"
        ));
    }

    let user_state: BorrowLendUserState = response_json(
        "borrowLendUserState",
        user_state_resp.map_err(|e| format!("borrowLendUserState request failed: {e}"))?,
    )
    .await?;

    let interest_entries: Vec<BorrowLendInterestEntry> = response_json(
        "userBorrowLendInterest",
        interest_resp.map_err(|e| format!("userBorrowLendInterest request failed: {e}"))?,
    )
    .await?;

    let token_name_by_id: HashMap<u32, String> = optional_response_value(spot_meta_resp)
        .await
        .map(|raw| parse_spot_token_names(&raw))
        .unwrap_or_default();

    Ok(build_income_snapshot(
        user_state,
        &interest_entries,
        &reserve_by_token,
        &token_name_by_id,
    ))
}

#[cfg(test)]
mod tests {
    use super::fetch_income_data_from_url;
    use serde_json::Value;
    use std::collections::HashMap;

    async fn income_server(responses: HashMap<&'static str, (&'static str, Value)>) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let responses: HashMap<String, (String, String)> = responses
            .into_iter()
            .map(|(ty, (status, body))| (ty.to_string(), (status.to_string(), body.to_string())))
            .collect();

        tokio::spawn(async move {
            for _ in 0..4 {
                let (mut socket, _) = listener.accept().await.expect("accept");
                let responses = responses.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0_u8; 8192];
                    let n = socket.read(&mut buf).await.expect("read request");
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
                    let ty = serde_json::from_str::<Value>(body)
                        .ok()
                        .and_then(|raw| raw.get("type").and_then(Value::as_str).map(str::to_string))
                        .unwrap_or_default();
                    let (status_line, response_body) =
                        responses.get(&ty).cloned().unwrap_or_else(|| {
                            (
                                "500 Internal Server Error".to_string(),
                                "unknown request".to_string(),
                            )
                        });
                    let response = format!(
                        "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
                        response_body.len()
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("write response");
                });
            }
        });

        format!("http://{addr}/info")
    }

    fn healthy_income_responses() -> HashMap<&'static str, (&'static str, Value)> {
        HashMap::from([
            (
                "allBorrowLendReserveStates",
                (
                    "200 OK",
                    serde_json::json!([[0, {"borrowYearlyRate": "0.01", "supplyYearlyRate": "0.02", "oraclePx": "1"}]]),
                ),
            ),
            (
                "borrowLendUserState",
                (
                    "200 OK",
                    serde_json::json!({
                        "tokenToState": [[0, {"borrow": {"value": "0"}, "supply": {"value": "100"}}]],
                        "health": "healthy",
                        "healthFactor": "10"
                    }),
                ),
            ),
            ("userBorrowLendInterest", ("200 OK", serde_json::json!([]))),
            (
                "spotMeta",
                (
                    "200 OK",
                    serde_json::json!({"tokens": [{"index": 0, "name": "USDC"}]}),
                ),
            ),
        ])
    }

    #[tokio::test]
    async fn income_required_endpoint_reports_http_status_before_json_parse() {
        let mut responses = healthy_income_responses();
        responses.insert(
            "borrowLendUserState",
            ("503 Service Unavailable", serde_json::json!("maintenance")),
        );
        let url = income_server(responses).await;

        let err = fetch_income_data_from_url(reqwest::Client::new(), &url, "0xabc".to_string())
            .await
            .expect_err("required endpoint HTTP failure should fail income fetch");

        assert!(
            err.contains("borrowLendUserState request failed with HTTP 503 Service Unavailable")
        );
        assert!(err.contains("maintenance"));
        assert!(!err.contains("parse failed"));
    }

    #[tokio::test]
    async fn income_ignores_non_success_spot_meta_and_keeps_snapshot() {
        let mut responses = healthy_income_responses();
        responses.insert(
            "spotMeta",
            ("429 Too Many Requests", serde_json::json!("rate limited")),
        );
        let url = income_server(responses).await;

        let snapshot =
            fetch_income_data_from_url(reqwest::Client::new(), &url, "0xabc".to_string())
                .await
                .expect("spotMeta should be best-effort");

        assert_eq!(snapshot.token_rows.len(), 1);
        assert_eq!(snapshot.token_rows[0].token_label, "#0");
        assert_eq!(snapshot.current_supply_usd, 100.0);
    }
}
