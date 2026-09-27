use super::super::SEC_FILING_SUMMARY_MAX_DOCUMENTS;
use super::*;

#[test]
fn sec_filing_document_url_uses_archive_document_path() {
    assert_eq!(
        sec_filing_document_url(1_652_044, "0001652044-26-000043", "goog-20260429.htm").as_deref(),
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
