use super::*;

#[test]
fn daily_flows_preserve_literal_dates_and_source_order_when_combining_funds() {
    let mut thyp = fund(HypeEtfTicker::Thyp, 1.0, 0.0);
    thyp.daily_flows = [
        ("2026-05-2", 1.0e16),
        ("2026-05-10", -0.0),
        ("2026-05-2", 1.0),
        ("invalid-only", f64::NAN),
        ("overflow", f64::MAX),
    ]
    .into_iter()
    .map(|(date, amount_usd)| HypeEtfDailyFlow {
        date: date.to_string(),
        amount_usd,
    })
    .collect();
    let mut bhyp = fund(HypeEtfTicker::Bhyp, 1.0, 0.0);
    bhyp.daily_flows = [
        ("2026-05-2", -1.0e16),
        ("2026-05-2", 2.0),
        ("invalid-only", f64::INFINITY),
        ("overflow", f64::MAX),
    ]
    .into_iter()
    .map(|(date, amount_usd)| HypeEtfDailyFlow {
        date: date.to_string(),
        amount_usd,
    })
    .collect();
    let data = HypeEtfData {
        funds: vec![thyp, bhyp],
        warnings: Vec::new(),
    };

    for (view, expected) in [
        (
            HypeEtfView::All,
            vec![
                ("2026-05-10", 0.0),
                ("2026-05-2", 2.0),
                ("overflow", f64::INFINITY),
            ],
        ),
        (
            HypeEtfView::Thyp,
            vec![
                ("2026-05-10", 0.0),
                ("2026-05-2", 1.0e16),
                ("overflow", f64::MAX),
            ],
        ),
        (
            HypeEtfView::Bhyp,
            vec![("2026-05-2", -1.0e16 + 2.0), ("overflow", f64::MAX)],
        ),
    ] {
        let flows = data.daily_flows_for(view);
        assert_eq!(flows.len(), expected.len());
        for (flow, (date, amount)) in flows.iter().zip(expected) {
            assert_eq!(flow.date, date);
            assert_eq!(flow.amount_usd.to_bits(), amount.to_bits());
        }
    }
    assert!(
        HypeEtfData::default()
            .daily_flows_for(HypeEtfView::All)
            .is_empty()
    );
}

#[test]
fn daily_flows_sum_by_date_for_selected_view() {
    let mut thyp = fund(HypeEtfTicker::Thyp, 1.0, 0.0);
    thyp.daily_flows = vec![
        HypeEtfDailyFlow {
            date: "2026-05-15".to_string(),
            amount_usd: 100.0,
        },
        HypeEtfDailyFlow {
            date: "2026-05-18".to_string(),
            amount_usd: 50.0,
        },
        HypeEtfDailyFlow {
            date: "2026-05-18".to_string(),
            amount_usd: f64::NAN,
        },
    ];
    let mut bhyp = fund(HypeEtfTicker::Bhyp, 1.0, 0.0);
    bhyp.daily_flows = vec![HypeEtfDailyFlow {
        date: "2026-05-15".to_string(),
        amount_usd: -25.0,
    }];
    let data = HypeEtfData {
        funds: vec![bhyp, thyp],
        warnings: Vec::new(),
    };

    assert_eq!(
        data.daily_flows_for(HypeEtfView::All),
        vec![
            HypeEtfDailyFlow {
                date: "2026-05-15".to_string(),
                amount_usd: 75.0,
            },
            HypeEtfDailyFlow {
                date: "2026-05-18".to_string(),
                amount_usd: 50.0,
            },
        ]
    );
    assert_eq!(
        data.daily_flows_for(HypeEtfView::Bhyp),
        vec![HypeEtfDailyFlow {
            date: "2026-05-15".to_string(),
            amount_usd: -25.0,
        }]
    );
}
