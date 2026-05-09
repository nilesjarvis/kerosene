use super::{API_URL, CLIENT};

mod model;
mod pagination;

pub use model::{UserFill, UserFillsPage, UserFillsRequest};
use pagination::next_user_fills_request;

pub async fn fetch_user_fills(
    address: String,
    request: UserFillsRequest,
) -> Result<UserFillsPage, String> {
    if request.end_time.is_some() {
        // Sleep 2 seconds between paginated API calls to avoid 429.
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    }

    let current_start = request.start_time;
    let end_time = request.end_time.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    });

    let client = CLIENT.clone();
    let body = serde_json::json!({
        "type": "userFillsByTime",
        "user": address,
        "startTime": current_start,
        "endTime": end_time,
        "aggregateByTime": false
    });

    let mut retries = 0;
    let response = loop {
        let resp = client
            .post(API_URL)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch user fills: {}", e))?;

        if resp.status().as_u16() == 429 {
            if retries >= 5 {
                return Err("API error: 429 Too Many Requests (max retries reached)".to_string());
            }
            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(5000 * retries)).await;
            continue;
        }

        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }

        break resp;
    };

    let mut fills: Vec<UserFill> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse user fills: {}", e))?;
    let fetched_count = fills.len();
    crate::journal::normalize_fills(&mut fills);

    if fills.is_empty() {
        return Ok(UserFillsPage {
            fills,
            next_request: None,
            requested_end_time: end_time,
        });
    }

    let oldest_time = fills.first().map(|f| f.time).unwrap_or(0);
    let newest_time = fills.last().map(|f| f.time).unwrap_or(0);
    let next_request =
        next_user_fills_request(current_start, fetched_count, oldest_time, newest_time);

    Ok(UserFillsPage {
        fills,
        next_request,
        requested_end_time: end_time,
    })
}
