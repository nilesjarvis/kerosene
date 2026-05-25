use super::*;

use iced::widget::pane_grid;

#[test]
fn boot_refresh_only_starts_when_hype_etf_pane_is_open() {
    let (mut terminal, _) = TradingTerminal::boot();
    let (panes, pane) = pane_grid::State::new(PaneKind::Chart(0));
    terminal.panes = panes;
    terminal.hype_etfs.loading = false;

    let _task = terminal.request_hype_etfs_boot_refresh();
    assert!(!terminal.hype_etfs.loading);

    terminal
        .panes
        .split(pane_grid::Axis::Vertical, pane, PaneKind::HypeEtfs)
        .expect("split should create HYPE ETF pane");

    let _task = terminal.request_hype_etfs_boot_refresh();
    assert!(terminal.hype_etfs.loading);
}
