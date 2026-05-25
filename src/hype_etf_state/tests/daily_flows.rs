use super::*;

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
