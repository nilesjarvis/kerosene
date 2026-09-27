use super::*;

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
