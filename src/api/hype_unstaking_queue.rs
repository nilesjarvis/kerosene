use super::CLIENT;
use crate::hype_unstaking_state::{HypeUnstakingEvent, HypeUnstakingQueueData};

use serde::Deserialize;

const HYPURRSCAN_UNSTAKING_QUEUE_URL: &str = "https://api.hypurrscan.io/unstakingQueue";

// ---------------------------------------------------------------------------
// HYPE Unstaking Queue API
// ---------------------------------------------------------------------------

pub(crate) async fn fetch_hype_unstaking_queue() -> Result<HypeUnstakingQueueData, String> {
    let response = CLIENT
        .clone()
        .get(HYPURRSCAN_UNSTAKING_QUEUE_URL)
        .send()
        .await
        .map_err(|e| format!("HYPE unstaking queue request failed: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("HYPE unstaking queue response read failed: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "HYPE unstaking queue request failed (HTTP {}): {}",
            status,
            response_snippet(&text)
        ));
    }

    let rows: Vec<HypurrscanUnstakingQueueRow> = serde_json::from_str(&text).map_err(|e| {
        format!(
            "HYPE unstaking queue response parse failed: {e}; {}",
            response_snippet(&text)
        )
    })?;

    Ok(HypeUnstakingQueueData::new(
        rows.into_iter().map(HypeUnstakingEvent::from).collect(),
    ))
}

#[derive(Debug, Clone, Deserialize)]
struct HypurrscanUnstakingQueueRow {
    time: u64,
    user: String,
    wei: u64,
}

impl From<HypurrscanUnstakingQueueRow> for HypeUnstakingEvent {
    fn from(row: HypurrscanUnstakingQueueRow) -> Self {
        Self {
            unlock_time_ms: row.time,
            user: row.user,
            amount_wei: row.wei,
        }
    }
}

fn response_snippet(text: &str) -> String {
    text.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hypurrscan_unstaking_rows() {
        let text = r#"
            [
                {
                    "time": 1779301327387,
                    "user": "0xf764939b589138dd1c75601b10a408c66ee68cbe",
                    "wei": 330083104
                },
                {
                    "time": 1779301804740,
                    "user": "0x2c64a1d5d602e7fb6d21da6211dcecc6e17a0649",
                    "wei": 200000000
                }
            ]
        "#;

        let rows: Vec<HypurrscanUnstakingQueueRow> =
            serde_json::from_str(text).expect("fixture should parse");
        let data =
            HypeUnstakingQueueData::new(rows.into_iter().map(HypeUnstakingEvent::from).collect());

        assert_eq!(data.events.len(), 2);
        assert_eq!(data.events[0].unlock_time_ms, 1779301327387);
        assert_eq!(
            data.events[0].user,
            "0xf764939b589138dd1c75601b10a408c66ee68cbe"
        );
        assert_eq!(data.events[0].amount_wei, 330083104);
    }
}
