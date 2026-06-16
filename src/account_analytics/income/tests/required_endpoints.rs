use super::super::fetch_income_data_from_url;
use super::fixtures::{healthy_income_responses, income_server};

#[tokio::test]
async fn income_required_endpoint_reports_http_status_before_json_parse() {
    let mut responses = healthy_income_responses();
    responses.insert(
        "borrowLendUserState",
        ("503 Service Unavailable", serde_json::json!("maintenance")),
    );
    let url = income_server(responses).await;

    let err =
        match fetch_income_data_from_url(reqwest::Client::new(), &url, "0xabc".to_string()).await {
            Ok(_) => panic!("required endpoint HTTP failure should fail income fetch"),
            Err(err) => err,
        };

    assert!(err.contains("borrowLendUserState request failed with HTTP 503 Service Unavailable"));
    assert!(err.contains("maintenance"));
    assert!(!err.contains("parse failed"));
}

#[tokio::test]
async fn income_empty_reserve_unicode_preview_does_not_panic() {
    let mut responses = healthy_income_responses();
    responses.insert(
        "allBorrowLendReserveStates",
        ("200 OK", serde_json::json!("€".repeat(100))),
    );
    let url = income_server(responses).await;

    let err =
        match fetch_income_data_from_url(reqwest::Client::new(), &url, "0xabc".to_string()).await {
            Ok(_) => panic!("unparseable reserve response should fail income fetch"),
            Err(err) => err,
        };

    assert!(err.contains("allBorrowLendReserveStates response had no parseable reserve entries"));
}
