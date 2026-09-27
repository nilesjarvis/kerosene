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
