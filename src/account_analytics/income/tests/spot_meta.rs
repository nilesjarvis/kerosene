use super::super::fetch_income_data_from_url;
use super::fixtures::{healthy_income_responses, income_server};

#[tokio::test]
async fn income_ignores_non_success_spot_meta_and_keeps_snapshot() {
    let mut responses = healthy_income_responses();
    responses.insert(
        "spotMeta",
        ("429 Too Many Requests", serde_json::json!("rate limited")),
    );
    let url = income_server(responses).await;

    let snapshot =
        match fetch_income_data_from_url(reqwest::Client::new(), &url, "0xabc".to_string()).await {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("spotMeta should be best-effort: {error}"),
        };

    assert_eq!(snapshot.token_rows.len(), 1);
    assert_eq!(snapshot.token_rows[0].token_label, "#0");
    assert_eq!(snapshot.current_supply_usd, 100.0);
}
