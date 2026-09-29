use super::*;

fn company_fact(
    start: &str,
    end: &str,
    value: f64,
    accession_number: &str,
    form: &str,
    filed: &str,
) -> SecCompanyFact {
    SecCompanyFact {
        start: start.to_string(),
        end: end.to_string(),
        val: Value::from(value),
        accn: accession_number.to_string(),
        form: form.to_string(),
        filed: filed.to_string(),
    }
}

fn insert_company_concept(
    facts: &mut SecCompanyFacts,
    name: &str,
    unit: &str,
    values: Vec<SecCompanyFact>,
) {
    facts
        .facts
        .entry("us-gaap".to_string())
        .or_default()
        .insert(
            name.to_string(),
            SecCompanyConcept {
                units: HashMap::from([(unit.to_string(), values)]),
            },
        );
}

#[test]
fn structured_earnings_select_current_quarter_and_comparable_prior_year() {
    let accession = "0000320193-26-000013";
    let mut facts = SecCompanyFacts::default();
    insert_company_concept(
        &mut facts,
        "RevenueFromContractWithCustomerExcludingAssessedTax",
        "USD",
        vec![
            company_fact(
                "2025-09-28",
                "2026-03-28",
                254_940_000_000.0,
                accession,
                "10-Q",
                "2026-05-01",
            ),
            company_fact(
                "2025-12-28",
                "2026-03-28",
                111_184_000_000.0,
                accession,
                "10-Q",
                "2026-05-01",
            ),
            company_fact(
                "2024-12-29",
                "2025-03-29",
                95_359_000_000.0,
                accession,
                "10-Q",
                "2026-05-01",
            ),
        ],
    );
    insert_company_concept(
        &mut facts,
        "EarningsPerShareDiluted",
        "USD/shares",
        vec![
            company_fact(
                "2025-12-28",
                "2026-03-28",
                2.01,
                accession,
                "10-Q",
                "2026-05-01",
            ),
            company_fact(
                "2024-12-29",
                "2025-03-29",
                1.65,
                accession,
                "10-Q",
                "2026-05-01",
            ),
        ],
    );
    insert_company_concept(
        &mut facts,
        "NetIncomeLoss",
        "USD",
        vec![
            company_fact(
                "2025-12-28",
                "2026-03-28",
                29_578_000_000.0,
                accession,
                "10-Q",
                "2026-05-01",
            ),
            company_fact(
                "2024-12-29",
                "2025-03-29",
                24_780_000_000.0,
                accession,
                "10-Q",
                "2026-05-01",
            ),
        ],
    );

    let earnings = extract_structured_earnings(&facts, "2026-04-30").expect("earnings");

    assert_eq!(earnings.source_form, "10-Q");
    assert_eq!(earnings.source_accession_number, accession);
    assert_eq!(earnings.period_end, "2026-03-28");
    assert_eq!(
        earnings.metrics,
        vec![
            SecEarningsMetric {
                label: "Revenue".to_string(),
                value: "$111.2B".to_string(),
                yoy_change: Some("+16.6% YoY".to_string()),
            },
            SecEarningsMetric {
                label: "Diluted EPS".to_string(),
                value: "$2.01".to_string(),
                yoy_change: Some("+21.8% YoY".to_string()),
            },
            SecEarningsMetric {
                label: "Net income".to_string(),
                value: "$29.6B".to_string(),
                yoy_change: Some("+19.4% YoY".to_string()),
            },
        ]
    );
}

#[test]
fn structured_earnings_reject_periodic_filing_too_far_after_event() {
    let mut facts = SecCompanyFacts::default();
    insert_company_concept(
        &mut facts,
        "Revenues",
        "USD",
        vec![company_fact(
            "2026-01-01",
            "2026-03-31",
            10_000_000_000.0,
            "0000000001-26-000001",
            "10-Q",
            "2026-06-20",
        )],
    );

    assert!(extract_structured_earnings(&facts, "2026-04-30").is_none());
}
