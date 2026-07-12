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
const SEC_ARCHIVES_BASE_URL: &str = "https://www.sec.gov/Archives/edgar/data";
const SEC_FILING_SUMMARY_MAX_DOCUMENTS: usize = 2;
const SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT: usize = 90_000;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecFilingSummaryRequest {
    pub(crate) cik: u64,
    pub(crate) accession_number: String,
    pub(crate) primary_document: String,
    pub(crate) form: String,
    pub(crate) filing_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecFilingSummary {
    pub(crate) form: String,
    pub(crate) filing_date: String,
    pub(crate) source_documents: Vec<String>,
    pub(crate) headline: Option<String>,
    pub(crate) highlights: Vec<String>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SecFilingDocument {
    document_type: String,
    filename: String,
    text: String,
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

pub(crate) async fn fetch_sec_filing_summary(
    request: SecFilingSummaryRequest,
) -> Result<SecFilingSummary, String> {
    let archive_url = sec_filing_archive_text_url(request.cik, &request.accession_number)
        .ok_or_else(|| "SEC filing archive URL unavailable".to_string())?;
    let archive_text = sec_get_text(&archive_url).await?;
    let filing_documents = parse_sec_filing_documents(&archive_text);
    let selected_documents = select_filing_summary_documents(
        &filing_documents,
        &request.primary_document,
        SEC_FILING_SUMMARY_MAX_DOCUMENTS,
    );
    if selected_documents.is_empty() {
        return Err("SEC filing package has no readable summary document".to_string());
    }

    let mut source_documents = Vec::new();
    let mut combined_text = String::new();
    for document in selected_documents {
        let text = html_to_plain_text(&document.text);
        if text.trim().is_empty() {
            continue;
        }
        source_documents.push(summary_source_document_label(document));
        if !combined_text.is_empty() {
            combined_text.push(' ');
        }
        combined_text.push_str(&text);
        if combined_text.len() >= SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT {
            combined_text.truncate(SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT);
            break;
        }
    }

    if combined_text.trim().is_empty() {
        return Err("SEC filing document text was empty".to_string());
    }

    let (headline, highlights) = summarize_filing_text(&combined_text);
    Ok(SecFilingSummary {
        form: request.form,
        filing_date: request.filing_date,
        source_documents,
        headline,
        highlights,
    })
}

pub(crate) fn sec_filing_document_url(
    cik: u64,
    accession_number: &str,
    primary_document: &str,
) -> Option<String> {
    if cik == 0 {
        return None;
    }

    let accession_digits = accession_digits(accession_number)?;
    let primary_document = primary_document.trim().trim_start_matches('/');
    if accession_digits.is_empty() || !safe_sec_document_path(primary_document) {
        return None;
    }

    Some(format!(
        "{SEC_ARCHIVES_BASE_URL}/{cik}/{accession_digits}/{primary_document}"
    ))
}

fn sec_filing_archive_text_url(cik: u64, accession_number: &str) -> Option<String> {
    if cik == 0 {
        return None;
    }
    let accession_digits = accession_digits(accession_number)?;
    let accession_number = accession_number.trim();
    if !safe_sec_accession_number(accession_number) {
        return None;
    }
    Some(format!(
        "{SEC_ARCHIVES_BASE_URL}/{cik}/{accession_digits}/{accession_number}.txt"
    ))
}

fn safe_sec_accession_number(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '-')
}

fn safe_sec_document_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

fn accession_digits(accession_number: &str) -> Option<String> {
    let digits = accession_number
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty()).then_some(digits)
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

async fn sec_get_text(url: &str) -> Result<String, String> {
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
        .text()
        .await
        .map_err(|e| format!("SEC text response parse failed: {e}"))
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
    } else if url.starts_with(SEC_ARCHIVES_BASE_URL) {
        "filing archive"
    } else {
        "EDGAR API"
    }
}

fn select_filing_summary_documents<'a>(
    documents: &'a [SecFilingDocument],
    primary_document: &str,
    limit: usize,
) -> Vec<&'a SecFilingDocument> {
    let primary = primary_document
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase();
    let mut scored = documents
        .iter()
        .filter_map(|item| {
            let name = item.filename.trim().trim_start_matches('/');
            if !filing_document_name_is_safe(name) || !filing_document_is_html(name) {
                return None;
            }
            let score = filing_summary_document_score(&item.document_type, name, &primary);
            (score > 0).then_some((score, item))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|(left_score, left_doc), (right_score, right_doc)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_doc.filename.cmp(&right_doc.filename))
    });

    scored
        .into_iter()
        .take(limit)
        .map(|(_, document)| document)
        .collect::<Vec<_>>()
}

fn filing_summary_document_score(document_type: &str, name: &str, primary_document: &str) -> i32 {
    let document_type = document_type.trim().to_ascii_lowercase();
    let lower = name.to_ascii_lowercase();
    let lower_stem = lower
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(lower.as_str());
    if lower.contains("-index")
        || lower.ends_with(".txt")
        || document_type == "xml"
        || lower_stem.starts_with('r')
            && lower_stem[1..]
                .chars()
                .all(|ch| ch.is_ascii_digit() || ch == '.')
    {
        return 0;
    }

    let mut score = 1;
    if document_type.starts_with("ex-99.1") || document_type == "ex99.1" {
        score += 120;
    } else if document_type.starts_with("ex-99") || document_type.starts_with("ex99") {
        score += 100;
    } else if document_type == "8-k" || document_type == "8k" {
        score += 8;
    }
    if lower == primary_document {
        score += 5;
    } else {
        score += 20;
    }
    for (needle, weight) in [
        ("ex99", 90),
        ("exhibit99", 90),
        ("99_1", 85),
        ("991", 80),
        ("press", 55),
        ("pr", 45),
        ("earn", 45),
        ("result", 40),
        ("commentary", 30),
        ("shareholder", 20),
    ] {
        if lower.contains(needle) {
            score += weight;
        }
    }
    score
}

fn parse_sec_filing_documents(archive_text: &str) -> Vec<SecFilingDocument> {
    let mut documents = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = archive_text[cursor..].find("<DOCUMENT>") {
        let start = cursor + relative_start + "<DOCUMENT>".len();
        let Some(relative_end) = archive_text[start..].find("</DOCUMENT>") else {
            break;
        };
        let end = start + relative_end;
        let block = &archive_text[start..end];
        if let Some(document) = parse_sec_filing_document_block(block) {
            documents.push(document);
        }
        cursor = end + "</DOCUMENT>".len();
    }
    documents
}

fn parse_sec_filing_document_block(block: &str) -> Option<SecFilingDocument> {
    let document_type = sec_document_header_value(block, "TYPE")?;
    let filename = sec_document_header_value(block, "FILENAME")
        .or_else(|| sec_document_header_value(block, "SEQUENCE"))?;
    let text_start = block
        .find("<TEXT>")
        .map(|index| index + "<TEXT>".len())
        .unwrap_or(0);
    let text_end = block[text_start..]
        .find("</TEXT>")
        .map(|index| text_start + index)
        .unwrap_or(block.len());
    let text = block[text_start..text_end].trim().to_string();
    if text.is_empty() || !filing_document_name_is_safe(&filename) {
        return None;
    }

    Some(SecFilingDocument {
        document_type,
        filename,
        text,
    })
}

fn sec_document_header_value(block: &str, name: &str) -> Option<String> {
    let marker = format!("<{name}>");
    let start = block.find(&marker)? + marker.len();
    let value = block[start..].lines().next()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn summary_source_document_label(document: &SecFilingDocument) -> String {
    if document.document_type.trim().is_empty() {
        document.filename.clone()
    } else {
        format!("{} {}", document.document_type, document.filename)
    }
}

fn filing_document_name_is_safe(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && !name.contains("://")
        && !name.contains('\\')
        && !name.contains("..")
        && !name.starts_with('/')
}

fn filing_document_is_html(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".htm") || lower.ends_with(".html")
}

fn html_to_plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len().min(SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT));
    let mut in_tag = false;
    let mut tag_buf = String::new();
    let mut skip_until: Option<&'static str> = None;
    let mut entity = String::new();
    let mut in_entity = false;

    for ch in html.chars() {
        if let Some(end_tag) = skip_until {
            tag_buf.push(ch.to_ascii_lowercase());
            if tag_buf.ends_with(end_tag) {
                skip_until = None;
                tag_buf.clear();
                text.push(' ');
            } else if tag_buf.len() > end_tag.len() + 32 {
                tag_buf = tag_buf
                    .chars()
                    .rev()
                    .take(end_tag.len())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
            }
            continue;
        }

        if in_tag {
            if ch == '>' {
                let tag = tag_buf.trim().to_ascii_lowercase();
                if tag.starts_with("script") {
                    skip_until = Some("</script>");
                    tag_buf.clear();
                } else if tag.starts_with("style") {
                    skip_until = Some("</style>");
                    tag_buf.clear();
                } else {
                    if tag.starts_with("br")
                        || tag.starts_with("/p")
                        || tag.starts_with("/div")
                        || tag.starts_with("/tr")
                        || tag.starts_with("/table")
                    {
                        text.push(' ');
                    }
                    tag_buf.clear();
                }
                in_tag = false;
            } else {
                tag_buf.push(ch);
            }
            continue;
        }

        if in_entity {
            if ch == ';' {
                text.push_str(&decode_html_entity(&entity));
                entity.clear();
                in_entity = false;
            } else if entity.len() < 16 {
                entity.push(ch);
            } else {
                text.push('&');
                text.push_str(&entity);
                entity.clear();
                in_entity = false;
                text.push(ch);
            }
            continue;
        }

        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
                text.push(' ');
            }
            '&' => {
                in_entity = true;
                entity.clear();
            }
            _ => text.push(ch),
        }
    }

    normalize_filing_text(&text)
}

fn decode_html_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" | "160" => " ".to_string(),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| format!("&{entity};"))
        }
        _ if entity.starts_with('#') => entity[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| format!("&{entity};")),
        _ => format!("&{entity};"),
    }
}

fn normalize_filing_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT));
    let mut last_was_space = true;
    for ch in text.chars() {
        let normalized = match ch {
            '\u{2013}' | '\u{2014}' => '-',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            _ => ch,
        };
        if normalized.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(normalized);
            last_was_space = false;
        }
    }
    out.trim().to_string()
}

fn summarize_filing_text(text: &str) -> (Option<String>, Vec<String>) {
    let headline = filing_headline(text);
    let mut highlights = Vec::new();
    for keywords in [
        &["revenue", "net sales", "sales"][..],
        &["net income", "net loss"][..],
        &["diluted", "eps", "earnings per share"][..],
        &["gross margin", "operating margin"][..],
        &["operating income", "operating loss"][..],
        &["cash flow", "free cash flow", "cash and equivalents"][..],
        &["guidance", "outlook", "expects", "forecast"][..],
        &["dividend", "repurchase", "buyback"][..],
    ] {
        if let Some(snippet) = first_relevant_snippet(text, keywords, &highlights) {
            highlights.push(snippet);
        }
        if highlights.len() >= 5 {
            break;
        }
    }

    if highlights.is_empty()
        && let Some(fallback) = fallback_filing_snippet(text, headline.as_deref())
    {
        highlights.push(fallback);
    }

    (headline, highlights)
}

fn filing_headline(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    for needle in ["reports", "announces", "financial results", "quarter"] {
        if let Some(index) = lower.find(needle) {
            let snippet = snippet_around(text, index, 90, 130);
            if snippet_has_numbers_or_reporting_words(&snippet) {
                return Some(trim_summary_snippet(&snippet, 132));
            }
        }
    }
    fallback_filing_snippet(text, None)
}

fn first_relevant_snippet(text: &str, keywords: &[&str], existing: &[String]) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let mut best: Option<(i32, String)> = None;
    for keyword in keywords {
        let mut cursor = 0;
        while let Some(relative_index) = lower[cursor..].find(keyword) {
            let index = cursor + relative_index;
            let snippet = trim_summary_snippet(&snippet_around(text, index, 70, 170), 150);
            cursor = index + keyword.len();
            if snippet_has_numbers_or_reporting_words(&snippet)
                && !snippet_is_boilerplate(&snippet)
                && !summary_snippet_seen(&snippet, existing)
            {
                let score = summary_snippet_score(&snippet, keywords);
                if best
                    .as_ref()
                    .is_none_or(|(best_score, _)| score > *best_score)
                {
                    best = Some((score, snippet));
                }
            }
        }
    }
    best.map(|(_, snippet)| snippet)
}

fn fallback_filing_snippet(text: &str, exclude: Option<&str>) -> Option<String> {
    for chunk in text.split(['.', ';']) {
        let snippet = trim_summary_snippet(chunk, 140);
        if snippet.len() > 35
            && !snippet_is_boilerplate(&snippet)
            && exclude
                .is_none_or(|excluded| !summary_snippet_seen(&snippet, &[excluded.to_string()]))
        {
            return Some(snippet);
        }
    }
    None
}

fn snippet_around(text: &str, index: usize, before: usize, after: usize) -> String {
    let start = previous_summary_delimiter(text, index)
        .map(|pos| pos + 1)
        .unwrap_or_else(|| previous_char_boundary(text, index.saturating_sub(before)));
    let end = next_summary_delimiter(text, index)
        .map(|pos| pos + 1)
        .unwrap_or_else(|| next_char_boundary(text, (index + after).min(text.len())));
    text[start..end].to_string()
}

fn previous_summary_delimiter(text: &str, before: usize) -> Option<usize> {
    let mut found = None;
    for (index, character) in text.char_indices() {
        if index >= before {
            break;
        }
        if summary_delimiter_at(text, index, character, true) {
            found = Some(index);
        }
    }
    found
}

fn next_summary_delimiter(text: &str, from: usize) -> Option<usize> {
    text.char_indices()
        .skip_while(|(index, _)| *index < from)
        .find_map(|(index, character)| {
            summary_delimiter_at(text, index, character, false).then_some(index)
        })
}

fn summary_delimiter_at(text: &str, index: usize, character: char, include_colon: bool) -> bool {
    match character {
        ';' => true,
        ':' => include_colon,
        '.' => {
            let previous_is_digit = text[..index]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit());
            let next_index = index + character.len_utf8();
            let next_is_digit = text[next_index..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit());
            !(previous_is_digit && next_is_digit)
        }
        _ => false,
    }
}

fn previous_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn trim_summary_snippet(snippet: &str, max_chars: usize) -> String {
    let snippet = normalize_filing_text(snippet)
        .trim_matches(['-', ':', ';', '.'])
        .trim()
        .to_string();
    if snippet.chars().count() <= max_chars {
        return snippet;
    }

    let mut out = snippet
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    while out.chars().last().is_some_and(char::is_whitespace) {
        out.pop();
    }
    out.push_str("...");
    out
}

fn snippet_has_numbers_or_reporting_words(snippet: &str) -> bool {
    snippet.chars().any(|ch| ch.is_ascii_digit())
        || snippet.to_ascii_lowercase().contains("quarter")
        || snippet.to_ascii_lowercase().contains("year")
}

fn summary_snippet_score(snippet: &str, keywords: &[&str]) -> i32 {
    let lower = snippet.to_ascii_lowercase();
    let mut score = 0;
    if snippet.contains('$') || lower.contains("billion") || lower.contains("million") {
        score += 30;
    }
    if snippet.contains('%') {
        score += 12;
    }
    if lower.contains("total revenue")
        || lower.contains("revenues increased")
        || lower.contains("revenue was")
        || lower.contains("net sales")
        || lower.contains("diluted eps")
        || lower.contains("diluted earnings per share")
        || lower.contains("earnings per share")
        || lower.contains("cash flow")
        || lower.contains("outlook")
        || lower.contains("expects")
        || lower.contains("guidance")
    {
        score += 25;
    }
    if lower.contains("gaap") || lower.contains("non-gaap") {
        score += 8;
    }
    for keyword in keywords {
        if lower.contains(keyword) {
            score += 4;
        }
    }
    if lower.contains("highlights 03") || lower.contains("photos & charts") {
        score -= 40;
    }
    if lower.contains("forward-looking") {
        score -= 50;
    }
    score
}

fn snippet_is_boilerplate(snippet: &str) -> bool {
    let lower = snippet.to_ascii_lowercase();
    lower.contains("forward-looking")
        || lower.contains("safe harbor")
        || lower.contains("non-gaap financial measures")
        || lower.contains("reconciliation")
        || lower.contains("conference call")
        || lower.contains("investor relations")
        || lower.contains("table of contents")
        || lower.contains("photos & charts")
        || lower.contains("additional information")
}

fn summary_snippet_seen(snippet: &str, existing: &[String]) -> bool {
    let normalized = snippet
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .take(80)
        .collect::<String>();
    existing.iter().any(|item| {
        item.chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .map(|ch| ch.to_ascii_lowercase())
            .take(80)
            .collect::<String>()
            == normalized
    })
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

    #[test]
    fn sec_filing_document_url_uses_archive_document_path() {
        assert_eq!(
            sec_filing_document_url(1_652_044, "0001652044-26-000043", "goog-20260429.htm")
                .as_deref(),
            Some(
                "https://www.sec.gov/Archives/edgar/data/1652044/000165204426000043/goog-20260429.htm"
            )
        );
    }

    #[test]
    fn sec_filing_document_url_rejects_incomplete_or_unsafe_inputs() {
        assert!(sec_filing_document_url(0, "0001652044-26-000043", "goog.htm").is_none());
        assert!(sec_filing_document_url(1, "", "goog.htm").is_none());
        assert!(sec_filing_document_url(1, "0001", "").is_none());
        assert!(sec_filing_document_url(1, "0001", "../index.htm").is_none());
        assert!(sec_filing_document_url(1, "0001", "https://example.com").is_none());
        assert!(sec_filing_document_url(1, "0001", "nested\\file.htm").is_none());
        assert!(sec_filing_document_url(1, "0001", "report.htm&calc.exe").is_none());
        assert!(sec_filing_document_url(1, "0001", "report.htm|more").is_none());
        assert!(sec_filing_document_url(1, "0001", "report.htm?download=1").is_none());
        assert!(sec_filing_document_url(1, "0001", "report.htm#section").is_none());
    }

    #[test]
    fn sec_filing_archive_text_url_uses_complete_submission_path() {
        assert_eq!(
            sec_filing_archive_text_url(1_045_810, "0001045810-26-000051").as_deref(),
            Some(
                "https://www.sec.gov/Archives/edgar/data/1045810/000104581026000051/0001045810-26-000051.txt"
            )
        );
    }

    #[test]
    fn filing_summary_document_selection_prefers_earnings_exhibits() {
        let archive_text = r#"
<SEC-DOCUMENT>sample
<DOCUMENT>
<TYPE>8-K
<SEQUENCE>1
<FILENAME>nvda-20260520.htm
<TEXT>
<html><body>Item 2.02 Results of Operations and Financial Condition.</body></html>
</TEXT>
</DOCUMENT>
<DOCUMENT>
<TYPE>EX-99.1
<SEQUENCE>2
<FILENAME>q1fy27pr.htm
<TEXT>
<html><body>NVIDIA reports revenue of $44.1 billion and diluted EPS of $0.76.</body></html>
</TEXT>
</DOCUMENT>
<DOCUMENT>
<TYPE>GRAPHIC
<SEQUENCE>3
<FILENAME>logo.jpg
<TEXT>binary</TEXT>
</DOCUMENT>
<DOCUMENT>
<TYPE>XML
<SEQUENCE>4
<FILENAME>R1.htm
<TEXT>
<html><body>Generated inline XBRL table artifact.</body></html>
</TEXT>
</DOCUMENT>
</SEC-DOCUMENT>
"#;

        let documents = parse_sec_filing_documents(archive_text);
        let selected = select_filing_summary_documents(
            &documents,
            "nvda-20260520.htm",
            SEC_FILING_SUMMARY_MAX_DOCUMENTS,
        );

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].document_type, "EX-99.1");
        assert_eq!(selected[0].filename, "q1fy27pr.htm");
        assert_eq!(selected[1].filename, "nvda-20260520.htm");
    }

    #[test]
    fn html_to_plain_text_decodes_entities_and_skips_hidden_blocks() {
        let text = html_to_plain_text(
            r#"<html><head><style>.x{}</style><script>alert(1)</script></head>
            <body><p>Revenue&nbsp;&amp;&nbsp;EPS&#58; $10B</p></body></html>"#,
        );

        assert_eq!(text, "Revenue & EPS: $10B");
    }

    #[test]
    fn summarize_filing_text_extracts_financial_highlights() {
        let text = "NVIDIA reports financial results for the first quarter of fiscal 2027. \
            Revenue was $44.1 billion, up 69% from a year ago. \
            GAAP diluted EPS was $0.76. \
            The company expects revenue to be $45.0 billion next quarter.";

        let (headline, highlights) = summarize_filing_text(text);

        assert!(
            headline
                .as_deref()
                .is_some_and(|item| item.contains("reports"))
        );
        assert!(
            highlights
                .iter()
                .any(|item| item.contains("Revenue was $44.1 billion")),
            "highlights: {highlights:?}"
        );
        assert!(highlights.iter().any(|item| item.contains("diluted EPS")));
        assert!(
            highlights
                .iter()
                .any(|item| item.contains("expects revenue"))
        );
    }

    #[test]
    fn summarize_filing_text_prefers_specific_financial_snippets() {
        let text = "Highlights 03 Financial Summary 04 Operational Summary 05 Outlook 10 Photos & Charts 11. \
            Financial Summary Q1-2026 YoY Total automotive revenues 16,234 16% Services and other revenue 3,745 42%. \
            Revenue Total quarterly revenue increased 16% YoY to $22.4B.";

        let (_, highlights) = summarize_filing_text(text);

        assert!(
            highlights
                .iter()
                .any(|item| item.contains("Total quarterly revenue increased 16% YoY to $22.4B")),
            "highlights: {highlights:?}"
        );
    }
}
