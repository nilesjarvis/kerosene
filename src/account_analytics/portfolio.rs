use super::model::{PortfolioBucket, PortfolioHistory};
use crate::api::{API_URL, CLIENT};

use serde_json::Value;

/// Fetch user portfolio history buckets from the `portfolio` info endpoint.
pub async fn fetch_portfolio_history(address: String) -> Result<PortfolioHistory, String> {
    let response = CLIENT
        .clone()
        .post(API_URL)
        .json(&serde_json::json!({"type": "portfolio", "user": address}))
        .send()
        .await
        .map_err(|e| format!("portfolio request failed: {e}"))?;

    let raw: Value = response
        .json()
        .await
        .map_err(|e| format!("portfolio parse failed: {e}"))?;

    if let Some(obj) = raw.as_object()
        && let Some(err) = obj.get("error").and_then(|v| v.as_str())
    {
        return Err(format!("portfolio error: {err}"));
    }

    let entries = raw.as_array().ok_or_else(|| {
        let preview = raw.to_string();
        let snippet = if preview.len() > 180 {
            format!("{}...", &preview[..180])
        } else {
            preview
        };
        format!("portfolio response was not an array: {snippet}")
    })?;

    let mut out = PortfolioHistory::default();

    for entry in entries {
        let Some(pair) = entry.as_array() else {
            continue;
        };
        if pair.len() != 2 {
            continue;
        }

        let Some(label) = pair[0].as_str() else {
            continue;
        };
        let Some(obj) = pair[1].as_object() else {
            continue;
        };

        out.buckets
            .insert(label.to_string(), parse_portfolio_bucket(obj));
    }

    Ok(out)
}

fn parse_portfolio_bucket(obj: &serde_json::Map<String, Value>) -> PortfolioBucket {
    let mut bucket = PortfolioBucket::default();
    let account_value_history = parse_history_points_with_stats(obj.get("accountValueHistory"));
    let pnl_history = parse_history_points_with_stats(obj.get("pnlHistory"));
    bucket
        .account_value_history
        .extend(account_value_history.points);
    bucket.pnl_history.extend(pnl_history.points);
    bucket.skipped_invalid_points =
        account_value_history.invalid_points + pnl_history.invalid_points;
    bucket.vlm = obj.get("vlm").and_then(value_as_f64);
    bucket.invalid_vlm = obj
        .get("vlm")
        .is_some_and(|value| value_as_f64(value).is_none());
    bucket
}

struct ParsedHistoryPoints {
    points: Vec<(u64, f64)>,
    invalid_points: usize,
}

#[cfg(test)]
fn parse_history_points(raw: Option<&Value>) -> Vec<(u64, f64)> {
    parse_history_points_with_stats(raw).points
}

fn parse_history_points_with_stats(raw: Option<&Value>) -> ParsedHistoryPoints {
    let Some(points) = raw.and_then(|v| v.as_array()) else {
        return ParsedHistoryPoints {
            points: Vec::new(),
            invalid_points: 0,
        };
    };

    let mut parsed = Vec::new();
    let mut invalid_points = 0_usize;
    for point in points {
        let Some(p) = point.as_array() else {
            continue;
        };
        if p.len() != 2 {
            continue;
        }

        let Some(ts) = p[0].as_u64() else {
            continue;
        };
        if ts == 0 {
            continue;
        }

        let Some(value) = value_as_f64(&p[1]) else {
            invalid_points += 1;
            continue;
        };
        parsed.push((ts, value));
    }

    ParsedHistoryPoints {
        points: parsed,
        invalid_points,
    }
}

fn value_as_f64(value: &Value) -> Option<f64> {
    let parsed = if let Some(text) = value.as_str() {
        text.trim().parse::<f64>().ok()?
    } else {
        value.as_f64()?
    };
    parsed.is_finite().then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::{parse_history_points, parse_portfolio_bucket};

    #[test]
    fn history_points_skip_malformed_numeric_values() {
        let raw = serde_json::json!([
            [1_000, "12.5"],
            [2_000, "bad"],
            [3_000, "NaN"],
            [4_000, 14.25],
            [0, "99"],
            ["bad-ts", "100"]
        ]);

        assert_eq!(
            parse_history_points(Some(&raw)),
            vec![(1_000, 12.5), (4_000, 14.25)]
        );
    }

    #[test]
    fn portfolio_bucket_tracks_skipped_points_and_invalid_volume() {
        let raw = serde_json::json!({
            "accountValueHistory": [
                [1_000, "100"],
                [2_000, "bad"]
            ],
            "pnlHistory": [
                [1_000, "1"],
                [2_000, "NaN"]
            ],
            "vlm": "bad"
        });
        let bucket = parse_portfolio_bucket(raw.as_object().expect("bucket object"));

        assert_eq!(bucket.account_value_history, vec![(1_000, 100.0)]);
        assert_eq!(bucket.pnl_history, vec![(1_000, 1.0)]);
        assert_eq!(bucket.skipped_invalid_points, 2);
        assert_eq!(bucket.vlm, None);
        assert!(bucket.invalid_vlm);
    }
}
