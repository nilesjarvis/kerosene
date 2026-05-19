use crate::account;
use crate::app_state::TradingTerminal;

fn cum_funding_since_open(since_open: &str) -> account::CumFunding {
    account::CumFunding {
        since_open: since_open.to_string(),
    }
}

#[test]
fn position_funding_pnl_inverts_clearinghouse_cum_funding() {
    let paid = cum_funding_since_open("12.34");
    let received = cum_funding_since_open("-5.67");

    assert_eq!(
        TradingTerminal::position_funding_pnl(Some(&paid)),
        Some(-12.34)
    );
    assert_eq!(
        TradingTerminal::position_funding_pnl(Some(&received)),
        Some(5.67)
    );
}
