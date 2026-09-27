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

#[test]
fn summary_preserves_exact_selection_order_case_and_fallbacks() {
    let cases: &[(&str, Option<&str>, &[&str])] = &[
        ("", None, &[]),
        (" \t\n", None, &[]),
        ("Short text", None, &[]),
        (
            "背景: Équipe REPORTS results for the year 2026. REVENUE was $8.5 million. \
             Net income was $1.25 million. GAAP diluted EPS was $0.10. Gross margin was 40%. \
             Operating income was $2 million. Cash flow was $3 million. Outlook expects growth of 5%.",
            Some("Équipe REPORTS results for the year 2026"),
            &[
                "REVENUE was $8.5 million",
                "Net income was $1.25 million",
                "GAAP diluted EPS was $0.10",
                "Gross margin was 40%",
                "Operating income was $2 million",
            ],
        ),
        (
            "OPERATIONS remain resilient across the entire group; \
             Operations remain resilient across the entire group; \
             Teams continue improving delivery throughout the organization.",
            Some("OPERATIONS remain resilient across the entire group"),
            &["Teams continue improving delivery throughout the organization"],
        ),
        (
            "Safe harbor revenue was $100 billion. Revenue was $2 million. \
             Revenue was $3 million. GAAP net income and diluted EPS were $1 and $0.50.",
            Some("GAAP net income and diluted EPS were $1 and $0"),
            &[
                "Revenue was $2 million",
                "GAAP net income and diluted EPS were $1 and $0.50",
            ],
        ),
    ];
    for &(text, expected_headline, expected_highlights) in cases {
        let (headline, highlights) = summarize_filing_text(text);
        assert_eq!(headline.as_deref(), expected_headline, "input: {text}");
        assert_eq!(highlights, expected_highlights, "input: {text}");
    }
}

#[test]
fn snippet_windows_preserve_decimal_and_unicode_boundaries() {
    for (text, before, after, expected) in [
        (
            "Before; Revenue was $1.25 billion. After",
            4,
            8,
            " Revenue was $1.25 billion.",
        ),
        (
            "Context: Revenue $2.5 million; next",
            4,
            8,
            " Revenue $2.5 million;",
        ),
        ("é東京abcRevenuexyz界🙂", 4, 12, "京abcRevenuexyz界"),
    ] {
        let index = text.find("Revenue").expect("fixture keyword");
        assert_eq!(snippet_around(text, index, before, after), expected);
    }
}

#[test]
fn duplicate_snippets_compare_only_the_first_eighty_ascii_alphanumerics() {
    let cases = [
        (
            "Café REVENUE: $10.50m".to_string(),
            "cafrevenue1050M".to_string(),
            true,
        ),
        ("Revenue".to_string(), "Revenues".to_string(), false),
        ("é東京".to_string(), "中å".to_string(), true),
        (String::new(), "!".to_string(), true),
        (String::new(), "1".to_string(), false),
        (
            format!("{}x", "A".repeat(80)),
            format!("{}y", "a".repeat(80)),
            true,
        ),
        (
            format!("{}x", "A".repeat(79)),
            format!("{}y", "a".repeat(79)),
            false,
        ),
        (
            format!("{}: X trailer", "A".repeat(79)),
            format!("{}x!!different", "a".repeat(79)),
            true,
        ),
    ];
    for (left, right, expected) in cases {
        assert_eq!(summary_snippets_match(&left, &right), expected);
    }
}
