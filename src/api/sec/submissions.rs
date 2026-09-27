use super::SecEarningsEvent;
use super::http::{SEC_SUBMISSIONS_BASE_URL, SEC_TICKER_MAP_URL, sec_get_json};
use chrono::NaiveDate;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
struct SecTickerEntry {
    cik_str: u64,
    ticker: String,
    title: String,
}

#[derive(Clone, Deserialize)]
struct SecCompanySubmissions {
    #[serde(default)]
    name: String,
    filings: SecCompanyFilings,
}

#[derive(Clone, Default, Deserialize)]
struct SecCompanyFilings {
    #[serde(default)]
    recent: SecRecentFilings,
}

#[derive(Clone, Default, Deserialize)]
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
mod tests;
