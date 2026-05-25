use super::*;
use crate::portfolio_state::PnlValueDisplayMode;

#[test]
fn portfolio_pnl_value_mode_updates_even_when_pnl_is_hidden() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.hide_pnl = true;
    terminal.portfolio.pnl_value_display_mode = PnlValueDisplayMode::Usd;

    let _ = terminal.update_portfolio_income(Message::SetPortfolioPnlValueMode(
        PnlValueDisplayMode::Percent,
    ));
    assert_eq!(
        terminal.portfolio.pnl_value_display_mode,
        PnlValueDisplayMode::Percent
    );

    let _ = terminal
        .update_portfolio_income(Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Usd));
    assert_eq!(
        terminal.portfolio.pnl_value_display_mode,
        PnlValueDisplayMode::Usd
    );
}
