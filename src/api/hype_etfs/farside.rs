use super::http::fetch_json;
use crate::hype_etf_state::HypeEtfDailyFlow;

use chrono::NaiveDate;
use serde::Deserialize;

const FARSIDE_HYP_PAGE_URL: &str = "https://farside.co.uk/wp-json/wp/v2/pages/56748";
const BHYP_CHART_LABEL: &str = r#""Bitwise (BHYP)""#;

// ---------------------------------------------------------------------------
// Farside BHYP Flow Scraping
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct WpPage {
    content: WpContent,
}

#[derive(Debug, Deserialize)]
struct WpContent {
    rendered: String,
}

pub(super) async fn fetch_farside_bhyp_flows() -> Result<Vec<HypeEtfDailyFlow>, String> {
    let page: WpPage = fetch_json(FARSIDE_HYP_PAGE_URL, "Farside BHYP flows").await?;
    parse_bhyp_flows_from_html(&page.content.rendered)
}

pub(super) fn parse_bhyp_flows_from_html(html: &str) -> Result<Vec<HypeEtfDailyFlow>, String> {
    let (cumulative, labels) = extract_chart_data(html)?;
    daily_flows_from_cumulative(&cumulative, &labels)
}

// ---------------------------------------------------------------------------
// Chart.js Data Extraction
// ---------------------------------------------------------------------------

fn extract_chart_data(html: &str) -> Result<(Vec<f64>, Vec<String>), String> {
    let cumulative = extract_bhyp_cumulative(html)?;
    let labels = extract_labels(html)?;

    if labels.len() != cumulative.len() {
        return Err(format!(
            "Farside BHYP: labels count ({}) != data count ({})",
            labels.len(),
            cumulative.len()
        ));
    }

    Ok((cumulative, labels))
}

fn extract_bhyp_cumulative(html: &str) -> Result<Vec<f64>, String> {
    let pos = html
        .find(BHYP_CHART_LABEL)
        .ok_or_else(|| "Farside BHYP: chart marker not found in response".to_string())?;

    let data_array = find_array_after_key(html, pos, r#""data""#, "data")?;
    let cumulative: Vec<f64> = serde_json::from_str(data_array)
        .map_err(|e| format!("Farside BHYP: data array parse failed: {e}"))?;

    if cumulative.is_empty() {
        return Err("Farside BHYP: empty cumulative data array".to_string());
    }
    if cumulative.iter().any(|value| !value.is_finite()) {
        return Err("Farside BHYP: cumulative data included a non-finite value".to_string());
    }

    Ok(cumulative)
}

fn extract_labels(html: &str) -> Result<Vec<String>, String> {
    let pos = html
        .find(BHYP_CHART_LABEL)
        .ok_or_else(|| "Farside BHYP: chart marker not found in response".to_string())?;

    let labels_array = find_last_array_before_key(html, pos, "labels", "labels")?;
    let labels: Vec<String> = serde_json::from_str(labels_array)
        .map_err(|e| format!("Farside BHYP: labels array parse failed: {e}"))?;

    if labels.is_empty() {
        return Err("Farside BHYP: empty labels array".to_string());
    }

    Ok(labels)
}

fn find_array_after_key<'a>(
    source: &'a str,
    from: usize,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    let key_start = source[from..]
        .find(key)
        .map(|offset| from + offset)
        .ok_or_else(|| format!("Farside BHYP: {label} array not found after marker"))?;
    array_at_key(source, key_start, key, label)
}

fn find_last_array_before_key<'a>(
    source: &'a str,
    before: usize,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    let mut search_end = before;
    while let Some(key_start) = source[..search_end].rfind(key) {
        if let Ok(array) = array_at_key(source, key_start, key, label) {
            return Ok(array);
        }
        search_end = key_start;
    }

    Err(format!(
        "Farside BHYP: {label} array not found before marker"
    ))
}

fn array_at_key<'a>(
    source: &'a str,
    key_start: usize,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    let colon = skip_ascii_whitespace(source, key_start + key.len());
    if source.as_bytes().get(colon) != Some(&b':') {
        return Err(format!("Farside BHYP: {label} field was missing ':'"));
    }

    let array_start = skip_ascii_whitespace(source, colon + 1);
    if source.as_bytes().get(array_start) != Some(&b'[') {
        return Err(format!("Farside BHYP: {label} field was not an array"));
    }

    extract_json_array(source, array_start, label)
}

fn extract_json_array<'a>(
    source: &'a str,
    array_start: usize,
    label: &str,
) -> Result<&'a str, String> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in source[array_start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = array_start + offset + ch.len_utf8();
                    return Ok(&source[array_start..end]);
                }
            }
            _ => {}
        }
    }

    Err(format!("Farside BHYP: unclosed {label} array"))
}

fn skip_ascii_whitespace(source: &str, mut index: usize) -> usize {
    while source
        .as_bytes()
        .get(index)
        .is_some_and(u8::is_ascii_whitespace)
    {
        index += 1;
    }
    index
}

// ---------------------------------------------------------------------------
// Flow Derivation
// ---------------------------------------------------------------------------

pub(super) fn daily_flows_from_cumulative(
    cumulative: &[f64],
    labels: &[String],
) -> Result<Vec<HypeEtfDailyFlow>, String> {
    if labels.len() != cumulative.len() {
        return Err(format!(
            "Farside BHYP: labels count ({}) != data count ({})",
            labels.len(),
            cumulative.len()
        ));
    }

    let mut flows = Vec::with_capacity(cumulative.len());
    let mut previous_cumulative: Option<f64> = None;

    for (i, &cum_value) in cumulative.iter().enumerate() {
        if !cum_value.is_finite() {
            return Err("Farside BHYP: cumulative data included a non-finite value".to_string());
        }

        let date = parse_farside_date(&labels[i])?;
        match previous_cumulative {
            Some(prev) => {
                let daily_flow = cum_value - prev;
                // Farside values are in US$ millions; convert to raw USD.
                let amount_usd = daily_flow * 1_000_000.0;
                if !amount_usd.is_finite() {
                    return Err("Farside BHYP: derived flow was non-finite".to_string());
                }
                flows.push(HypeEtfDailyFlow { date, amount_usd });
            }
            None if cum_value.abs() <= f64::EPSILON => {
                flows.push(HypeEtfDailyFlow {
                    date,
                    amount_usd: 0.0,
                });
            }
            None => {}
        }
        previous_cumulative = Some(cum_value);
    }

    Ok(flows)
}

// ---------------------------------------------------------------------------
// Date Parsing: "12 May 2026" -> "2026-05-12"
// ---------------------------------------------------------------------------

pub(super) fn parse_farside_date(raw: &str) -> Result<String, String> {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(format!("Farside BHYP: invalid date label {raw:?}"));
    }

    let month = match parts[1] {
        "Jan" | "January" => 1,
        "Feb" | "February" => 2,
        "Mar" | "March" => 3,
        "Apr" | "April" => 4,
        "May" => 5,
        "Jun" | "June" => 6,
        "Jul" | "July" => 7,
        "Aug" | "August" => 8,
        "Sep" | "September" => 9,
        "Oct" | "October" => 10,
        "Nov" | "November" => 11,
        "Dec" | "December" => 12,
        _ => return Err(format!("Farside BHYP: invalid date label {raw:?}")),
    };
    let day = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("Farside BHYP: invalid date label {raw:?}"))?;
    let year = parts[2]
        .parse::<i32>()
        .map_err(|_| format!("Farside BHYP: invalid date label {raw:?}"))?;
    let date = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| format!("Farside BHYP: invalid date label {raw:?}"))?;

    Ok(date.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests;
