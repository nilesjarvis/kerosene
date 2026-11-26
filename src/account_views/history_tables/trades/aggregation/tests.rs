use super::*;

fn fill(oid: Option<u64>, time: u64) -> UserFill {
    UserFill {
        coin: "BTC".into(),
        px: "100".into(),
        sz: "1".into(),
        side: "B".into(),
        start_position: None,
        time,
        hash: None,
        tid: None,
        oid,
        dir: "Close Short".into(),
        closed_pnl: "2".into(),
        fee: "0.50".into(),
        fee_token: Some("USDC".into()),
    }
}

#[test]
fn sweep_combines_interleaved_fills_at_the_size_weighted_price() {
    let first = fill(Some(42), 10);
    let second = UserFill {
        px: "104".into(),
        sz: "3".into(),
        dir: "Open Long".into(),
        closed_pnl: "0".into(),
        fee: "-0.25".into(),
        fee_token: None,
        ..first.clone()
    };
    let fills = [fill(Some(42), 9), first, fill(Some(43), 10), second];
    let rows = trade_history_rows(fills.iter(), true);
    assert_eq!(rows.len(), 3);
    assert_eq!((rows[0].count(), rows[0].direction()), (2, "Mixed"));
    let amounts = rows[0].amounts();
    assert_eq!(amounts.price, Some(103.0));
    assert_eq!(amounts.size, Some(4.0));
    assert_eq!(amounts.pnl, Some(2.0));
    assert_eq!(amounts.fee, Some(0.25));
    assert_eq!(rows[1].first.oid, Some(43));
    assert_eq!(rows[2].first.time, 9);

    let raw = trade_history_rows(fills.iter(), false);
    assert_eq!(raw.len(), fills.len());
    for (row, fill) in raw.iter().zip(&fills) {
        assert_eq!(row.count(), 1);
        assert!(std::ptr::eq(row.first, fill));
    }
}

#[test]
fn different_markets_sides_fee_tokens_and_unidentified_fills_stay_separate() {
    let base = fill(Some(1), 10);
    let fills = [
        base.clone(),
        UserFill {
            coin: "ETH".into(),
            ..base.clone()
        },
        UserFill {
            side: "A".into(),
            ..base.clone()
        },
        UserFill {
            fee_token: Some("HYPE".into()),
            ..base
        },
        fill(None, 10),
        fill(None, 10),
        fill(Some(0), 10),
        fill(Some(0), 10),
    ];
    let rows = trade_history_rows(fills.iter(), true);
    assert_eq!(rows.len(), fills.len());
    assert!(rows.iter().all(|row| row.count() == 1));
}

#[test]
fn invalid_values_affect_only_their_own_totals() {
    let fills = [
        fill(Some(1), 10),
        UserFill {
            px: "NaN".into(),
            fee: "bad".into(),
            ..fill(Some(1), 10)
        },
    ];
    let amounts = trade_history_rows(fills.iter(), true)[0].amounts();
    assert_eq!(amounts.price, None);
    assert_eq!(amounts.fee, None);
    assert_eq!(amounts.size, Some(2.0));
    assert_eq!(amounts.pnl, Some(4.0));
}

#[test]
fn display_limit_counts_groups_and_includes_late_partials_of_the_last_visible_order() {
    let mut fills: Vec<_> = (1..=60).map(|oid| fill(Some(oid), 10)).collect();
    fills.extend((0..65).map(|_| fill(Some(50), 10)));
    let rows = trade_history_rows(fills.iter(), true);
    assert_eq!(rows.len(), ROW_LIMIT);
    assert_eq!(rows[49].count(), 66);
    assert_eq!(rows[49].amounts().size, Some(66.0));
    assert_eq!(trade_history_rows(fills.iter(), false).len(), ROW_LIMIT);
}
