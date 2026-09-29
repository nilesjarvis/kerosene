mod documents;
mod earnings;
mod http;
mod submissions;
mod summary;

pub(crate) use documents::sec_filing_document_url;
pub(crate) use submissions::fetch_sec_earnings_events;

use documents::{
    parse_sec_filing_documents, sec_filing_archive_text_url, select_filing_summary_documents,
    summary_source_document_label,
};
use earnings::{SecCompanyFacts, extract_structured_earnings};
use http::{SEC_COMPANY_FACTS_BASE_URL, sec_get_json, sec_get_text};
use summary::{html_to_plain_text, summarize_filing_text};

// ---------------------------------------------------------------------------
// SEC EDGAR Data API
// ---------------------------------------------------------------------------

const SEC_FILING_SUMMARY_MAX_DOCUMENTS: usize = 2;
const SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT: usize = 90_000;

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
    pub(crate) structured_earnings: Option<SecStructuredEarnings>,
    pub(crate) headline: Option<String>,
    pub(crate) highlights: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecStructuredEarnings {
    pub(crate) source_form: String,
    pub(crate) source_accession_number: String,
    pub(crate) period_end: String,
    pub(crate) metrics: Vec<SecEarningsMetric>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecEarningsMetric {
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) yoy_change: Option<String>,
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
    let company_facts_url = format!("{SEC_COMPANY_FACTS_BASE_URL}/CIK{:010}.json", request.cik);
    let structured_earnings = sec_get_json::<SecCompanyFacts>(&company_facts_url)
        .await
        .ok()
        .and_then(|facts| extract_structured_earnings(&facts, &request.filing_date));
    Ok(SecFilingSummary {
        form: request.form,
        filing_date: request.filing_date,
        source_documents,
        structured_earnings,
        headline,
        highlights,
    })
}
