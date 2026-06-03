use super::CLIENT;
use chrono::NaiveDate;
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// SEC EDGAR Data API
// ---------------------------------------------------------------------------

const SEC_TICKER_MAP_URL: &str = "https://www.sec.gov/files/company_tickers.json";
const SEC_SUBMISSIONS_BASE_URL: &str = "https://data.sec.gov/submissions";
const DEFAULT_SEC_USER_AGENT: &str = concat!(
    "Kerosene/",
    env!("CARGO_PKG_VERSION"),
    " sec-edgar@kerosene.local"
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecEarningsEvent {
    pub(crate) ticker: String,
    pub(crate) company_name: String,
    pub(crate) cik: u64,
    pub(crate) filing_date: String,
    pub(crate) filing_time_ms: u64,
    pub(crate) report_date: Option<String>,
    pub(crate) form: String,
    pub(crate) accession_number: String,
    pub(crate) primary_document: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SecTickerEntry {
    cik_str: u64,
    ticker: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SecCompanySubmissions {
    #[serde(default)]
    name: String,
    filings: SecCompanyFilings,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct SecCompanyFilings {
    #[serde(default)]
    recent: SecRecentFilings,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct SecRecentFilings {
    #[serde(default)]
    form: Vec<String>,
    #[serde(default, rename = "filingDate")]
    filing_date: Vec<String>,
    #[serde(default, rename = "reportDate")]
    report_date: Vec<String>,
    #[serde(default, rename = "accessionNumber")]
    accession_number: Vec<String>,
    #[serde(default, rename = "primaryDocument")]
    primary_document: Vec<String>,
    #[serde(default)]
    items: Vec<String>,
}

pub(crate) async fn fetch_sec_earnings_events(
    ticker: String,
) -> Result<Vec<SecEarningsEvent>, String> {
    let ticker = normalize_sec_ticker(&ticker)
        .ok_or_else(|| "SEC earnings ticker cannot be empty".to_string())?;
    let company = fetch_sec_ticker_entry(&ticker).await?;
    let submissions = fetch_sec_company_submissions(company.cik_str).await?;
    Ok(earnings_events_from_submissions(
        &ticker,
        &company,
        &submissions,
    ))
}

async fn fetch_sec_ticker_entry(ticker: &str) -> Result<SecTickerEntry, String> {
    let entries: HashMap<String, SecTickerEntry> = sec_get_json(SEC_TICKER_MAP_URL).await?;
    entries
        .into_values()
        .find(|entry| entry.ticker.eq_ignore_ascii_case(ticker))
        .ok_or_else(|| format!("SEC CIK not found for {ticker}"))
}

async fn fetch_sec_company_submissions(cik: u64) -> Result<SecCompanySubmissions, String> {
    let url = format!("{SEC_SUBMISSIONS_BASE_URL}/CIK{cik:010}.json");
    sec_get_json(&url).await
}

async fn sec_get_json<T>(url: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = CLIENT
        .get(url)
        .header(USER_AGENT, sec_user_agent())
        .send()
        .await
        .map_err(|e| format!("SEC request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "SEC request to {} returned {status}",
            sec_endpoint_label(url)
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("SEC response parse failed: {e}"))
}

fn sec_user_agent() -> String {
    std::env::var("KEROSENE_SEC_USER_AGENT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_SEC_USER_AGENT.to_string())
}

fn sec_endpoint_label(url: &str) -> &'static str {
    if url == SEC_TICKER_MAP_URL {
        "ticker map"
    } else if url.starts_with(SEC_SUBMISSIONS_BASE_URL) {
        "company submissions"
    } else {
        "EDGAR API"
    }
}

fn earnings_events_from_submissions(
    ticker: &str,
    company: &SecTickerEntry,
    submissions: &SecCompanySubmissions,
) -> Vec<SecEarningsEvent> {
    let mut events = Vec::new();
    for index in 0..submissions.filings.recent.form.len() {
        let recent = &submissions.filings.recent;
        let form = recent.form[index].trim();
        if form != "8-K" {
            continue;
        }

        let items = recent
            .items
            .get(index)
            .map(String::as_str)
            .unwrap_or_default();
        if !sec_items_contains(items, "2.02") {
            continue;
        }

        let filing_date = recent
            .filing_date
            .get(index)
            .map(|date| date.trim())
            .unwrap_or_default();
        let Some(filing_time_ms) = sec_date_to_unix_ms(filing_date) else {
            continue;
        };

        let report_date = recent
            .report_date
            .get(index)
            .map(|date| date.trim())
            .filter(|date| !date.is_empty())
            .map(str::to_string);

        events.push(SecEarningsEvent {
            ticker: ticker.to_string(),
            company_name: if submissions.name.trim().is_empty() {
                company.title.clone()
            } else {
                submissions.name.clone()
            },
            cik: company.cik_str,
            filing_date: filing_date.to_string(),
            filing_time_ms,
            report_date,
            form: form.to_string(),
            accession_number: recent
                .accession_number
                .get(index)
                .cloned()
                .unwrap_or_default(),
            primary_document: recent
                .primary_document
                .get(index)
                .cloned()
                .unwrap_or_default(),
        });
    }

    events.sort_by_key(|event| event.filing_time_ms);
    events
}

fn normalize_sec_ticker(ticker: &str) -> Option<String> {
    let ticker = ticker.trim();
    (!ticker.is_empty()).then(|| ticker.to_ascii_uppercase())
}

fn sec_items_contains(items: &str, expected: &str) -> bool {
    items
        .split(',')
        .map(str::trim)
        .any(|item| item.eq_ignore_ascii_case(expected))
}

fn sec_date_to_unix_ms(date: &str) -> Option<u64> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let datetime = date.and_hms_opt(0, 0, 0)?.and_utc();
    u64::try_from(datetime.timestamp_millis()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn company() -> SecTickerEntry {
        SecTickerEntry {
            cik_str: 1_652_044,
            ticker: "GOOGL".to_string(),
            title: "Alphabet Inc.".to_string(),
        }
    }

    #[test]
    fn earnings_events_include_only_8k_item_202_filings() {
        let submissions = SecCompanySubmissions {
            name: "Alphabet Inc.".to_string(),
            filings: SecCompanyFilings {
                recent: SecRecentFilings {
                    form: vec![
                        "8-K".to_string(),
                        "10-Q".to_string(),
                        "8-K".to_string(),
                        "8-K/A".to_string(),
                    ],
                    filing_date: vec![
                        "2026-04-29".to_string(),
                        "2026-04-30".to_string(),
                        "2026-04-10".to_string(),
                        "2026-05-01".to_string(),
                    ],
                    report_date: vec![
                        "2026-04-29".to_string(),
                        "2026-03-31".to_string(),
                        "2026-04-07".to_string(),
                        "2026-04-29".to_string(),
                    ],
                    accession_number: vec![
                        "0001652044-26-000043".to_string(),
                        "0001652044-26-000048".to_string(),
                        "0001652044-26-000034".to_string(),
                        "0001652044-26-000050".to_string(),
                    ],
                    primary_document: vec![
                        "goog-20260429.htm".to_string(),
                        "goog-20260331.htm".to_string(),
                        "goog-20260407.htm".to_string(),
                        "goog-20260501.htm".to_string(),
                    ],
                    items: vec![
                        "2.02,9.01".to_string(),
                        String::new(),
                        "5.02".to_string(),
                        "2.02".to_string(),
                    ],
                },
            },
        };

        let events = earnings_events_from_submissions("GOOGL", &company(), &submissions);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].ticker, "GOOGL");
        assert_eq!(events[0].filing_date, "2026-04-29");
        assert_eq!(events[0].report_date.as_deref(), Some("2026-04-29"));
        assert_eq!(events[0].accession_number, "0001652044-26-000043");
    }

    #[test]
    fn earnings_events_are_sorted_oldest_first() {
        let submissions = SecCompanySubmissions {
            name: String::new(),
            filings: SecCompanyFilings {
                recent: SecRecentFilings {
                    form: vec!["8-K".to_string(), "8-K".to_string()],
                    filing_date: vec!["2026-04-29".to_string(), "2026-02-04".to_string()],
                    report_date: vec![String::new(), String::new()],
                    accession_number: vec!["later".to_string(), "earlier".to_string()],
                    primary_document: vec![String::new(), String::new()],
                    items: vec!["2.02".to_string(), "2.02,9.01".to_string()],
                },
            },
        };

        let events = earnings_events_from_submissions("GOOGL", &company(), &submissions);

        assert_eq!(
            events
                .iter()
                .map(|event| event.accession_number.as_str())
                .collect::<Vec<_>>(),
            vec!["earlier", "later"]
        );
        assert_eq!(events[0].company_name, "Alphabet Inc.");
    }

    #[test]
    fn sec_date_to_unix_ms_parses_utc_midnight() {
        assert_eq!(sec_date_to_unix_ms("1970-01-02"), Some(86_400_000));
        assert_eq!(sec_date_to_unix_ms("not-a-date"), None);
    }
}
