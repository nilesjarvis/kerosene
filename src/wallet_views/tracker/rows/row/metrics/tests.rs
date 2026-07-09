use super::*;
use crate::account::WalletTrackerSnapshot;

#[test]
fn wallet_row_metrics_show_placeholders_without_snapshot() {
    let row = WalletTrackerRow::default();
    let denomination = DisplayDenominationContext::default();
    let metrics = wallet_row_metrics(&row, &denomination, &Theme::Dark);

    assert_eq!(metrics.equity, "-");
    assert_eq!(metrics.withdrawable, "-");
    assert_eq!(metrics.upnl, "-");
    assert_eq!(metrics.margin, "-");
    assert_eq!(metrics.risk, "-");
}

#[test]
fn wallet_row_metrics_mark_invalid_snapshot_values() {
    let row = WalletTrackerRow {
        snapshot: Some(WalletTrackerSnapshot {
            equity: None,
            withdrawable: Some(10.0),
            unrealized_pnl: None,
            margin_used_pct: None,
            open_trade_count: None,
            open_order_count: 0,
            long_exposure: None,
            short_exposure: Some(0.0),
            valuation_warning: None,
        }),
        ..WalletTrackerRow::default()
    };

    let denomination = DisplayDenominationContext::default();
    let metrics = wallet_row_metrics(&row, &denomination, &Theme::Dark);

    assert_eq!(metrics.equity, "Invalid data");
    assert_eq!(metrics.withdrawable, "$10.00");
    assert_eq!(metrics.upnl, "Invalid data");
    assert_eq!(metrics.margin, "Invalid data");
    assert_eq!(metrics.risk, "Invalid data / -o | Invalid data");
}
