use super::*;
use crate::journal::{FillIdentity, JournalAttributedFill};

pub(super) fn candle(open_time: u64, close_time: u64, low: f64, high: f64, close: f64) -> Candle {
    Candle::test_ohlcv(open_time, close_time, [close, high, low, close], 1.0)
}

pub(super) fn trade(is_long: bool) -> AggregatedTrade {
    AggregatedTrade {
        id: "perp:BTC:test".to_string(),
        legacy_note_ids: Vec::new(),
        coin: "BTC".to_string(),
        start_time: 1_000,
        end_time: Some(2_000),
        max_position: 1.0,
        volume: 0.0,
        fee: 0.0,
        pnl: 0.0,
        status: "CLOSED".to_string(),
        fill_count: 2,
        avg_entry_price: 100.0,
        total_entry_notional: 100.0,
        total_entry_size: 1.0,
        is_long,
        basis_complete: true,
    }
}

pub(super) fn details() -> JournalTradeDetails {
    JournalTradeDetails {
        trade_id: "perp:BTC:test".to_string(),
        coin: "BTC".to_string(),
        attributed_fills: vec![JournalAttributedFill {
            identity: FillIdentity {
                time: 2_000,
                tid: 1,
                oid: 1,
                hash: "0x1".to_string(),
                coin: "BTC".to_string(),
                side: "A".to_string(),
                px: "110".to_string(),
                sz: "1".to_string(),
            },
            time_ms: 2_000,
            price: 110.0,
            raw_size: 1.0,
            attributed_size: 1.0,
            side: "A".to_string(),
            role: JournalAttributedFillRole::Reduce,
            fee: 0.0,
            closed_pnl: 10.0,
        }],
    }
}

pub(super) fn request() -> JournalTradeSnapshotRequest {
    JournalTradeSnapshotRequest {
        account_key: Some("acct".to_string()),
        address: "0xabc".to_string(),
        trade_id: "perp:BTC:test".to_string(),
        coin: "BTC".to_string(),
        source: ChartBackfillSource::Hyperliquid,
        read_data_provider_generation: 0,
        hydromancer_key_generation: 0,
        coverage: JournalSnapshotCoverage::default(),
        timeframe: Timeframe::M1,
        ladder_index: 0,
        trade_start_ms: 1_000,
        trade_end_ms: 2_000,
        is_open: false,
        start_ms: 0,
        end_ms: 3_000,
    }
}

pub(super) fn live_trade() -> AggregatedTrade {
    let mut trade = trade(true);
    trade.id = "position:BTC".to_string();
    trade.end_time = None;
    trade.status = "OPEN".to_string();
    trade.fill_count = 0;
    trade
}

#[test]
fn request_debug_redacts_account_identifiers() {
    let rendered = format!("{:?}", request());

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains("acct"));
    assert!(!rendered.contains("0xabc"));
    assert!(rendered.contains("perp:BTC:test"));
    assert!(rendered.contains("BTC"));
}

#[test]
fn build_marks_open_position_without_fills_as_live() {
    let snapshot = build_journal_trade_snapshot(
        &request(),
        &live_trade(),
        None,
        vec![candle(0, 60_000, 95.0, 115.0, 110.0)],
    )
    .expect("snapshot");

    assert!(snapshot.live_position);
    assert!(snapshot.markers.is_empty());
}
