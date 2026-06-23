use super::*;

#[test]
fn chrome_layout_calendar_and_portfolio_routes_cover_shared_shell_messages() {
    assert_route(Message::Tick, UpdateRoute::Calendar);
    assert_route(
        Message::CalendarImpactFilterChanged(crate::calendar_state::CalendarImpactFilter::All),
        UpdateRoute::Chrome,
    );
    assert_route(Message::ToggleHidePnl, UpdateRoute::Chrome);
    assert_route(Message::ConfigSaved(Ok(())), UpdateRoute::Chrome);
    assert_route(Message::EnterApplication, UpdateRoute::Chrome);
    assert_route(
        Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Percent),
        UpdateRoute::PortfolioIncome,
    );
    assert_route(Message::RefreshPortfolio, UpdateRoute::PortfolioIncome);
    assert_route(Message::ToggleLayoutMenu, UpdateRoute::Panes);
    assert_route(Message::UpdateActiveLayout, UpdateRoute::Layout);
    assert_route(Message::LayoutRenameToggled(0), UpdateRoute::Layout);
    assert_route(
        Message::LayoutRenameChanged("Main".to_string()),
        UpdateRoute::Layout,
    );
    assert_route(Message::LayoutRenameSubmitted(0), UpdateRoute::Layout);
}
