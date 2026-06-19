use super::*;
use crate::api::UserFill;

mod aggregation;
mod fills;
mod notes;
mod state;

fn fill(time: u64, tid: u64, coin: &str) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: "100.0".to_string(),
        sz: "1.0".to_string(),
        side: "B".to_string(),
        time,
        start_position: "0.0".to_string(),
        dir: "Open Long".to_string(),
        closed_pnl: "0.0".to_string(),
        hash: format!("0x{time:x}{tid:x}"),
        oid: tid + 100,
        crossed: false,
        fee: "0.01".to_string(),
        tid,
        fee_token: "USDC".to_string(),
    }
}

fn wallet_hype_fill(
    time: u64,
    tid: u64,
    side: &str,
    dir: &str,
    sz: &str,
    start_position: &str,
    closed_pnl: &str,
) -> UserFill {
    UserFill {
        coin: "HYPE".to_string(),
        px: "40.0".to_string(),
        sz: sz.to_string(),
        side: side.to_string(),
        time,
        start_position: start_position.to_string(),
        dir: dir.to_string(),
        closed_pnl: closed_pnl.to_string(),
        hash: format!("0x{tid:x}"),
        oid: 422_000_000_000,
        crossed: false,
        fee: "0.0".to_string(),
        tid,
        fee_token: "USDC".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn spot_fill(
    time: u64,
    tid: u64,
    oid: u64,
    coin: &str,
    side: &str,
    px: &str,
    sz: &str,
    fee: &str,
    fee_token: &str,
    closed_pnl: &str,
) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: px.to_string(),
        sz: sz.to_string(),
        side: side.to_string(),
        time,
        start_position: "0.0".to_string(),
        dir: if side == "A" { "Sell" } else { "Buy" }.to_string(),
        closed_pnl: closed_pnl.to_string(),
        hash: format!("0x{time:x}{tid:x}{oid:x}"),
        oid,
        crossed: false,
        fee: fee.to_string(),
        tid,
        fee_token: fee_token.to_string(),
    }
}

fn assert_approx_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-6,
        "expected {actual} to be within 1e-6 of {expected}"
    );
}

fn note(open: &str) -> JournalNote {
    JournalNote {
        open: open.to_string(),
        close: String::new(),
    }
}

#[test]
fn journal_note_debug_summarizes_private_text() {
    let note = JournalNote {
        open: "private open note sentinel".to_string(),
        close: "private close note sentinel".to_string(),
    };

    let rendered = format!("{note:?}");

    assert!(rendered.contains("open: len=26"));
    assert!(rendered.contains("close: len=27"));
    assert!(!rendered.contains("private open note sentinel"));
    assert!(!rendered.contains("private close note sentinel"));
}
