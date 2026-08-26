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
        Message::ToggleWindowTransparency(true),
        UpdateRoute::Preferences,
    );
    assert_route(
        Message::ToggleWindowBackgroundBlur(true),
        UpdateRoute::Preferences,
    );
    assert_route(
        Message::WindowBackgroundOpacityChanged(0.7),
        UpdateRoute::Preferences,
    );
    assert_route(
        Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Percent),
        UpdateRoute::PortfolioIncome,
    );
    assert_route(Message::RefreshPortfolio, UpdateRoute::PortfolioIncome);
    assert_route(
        Message::SetIncomePaneView(crate::portfolio_state::IncomePaneView::Tokens),
        UpdateRoute::PortfolioIncome,
    );
    assert_route(Message::ToggleLayoutMenu, UpdateRoute::Panes);
    assert_route(Message::UpdateActiveLayout, UpdateRoute::Layout);
    assert_route(
        Message::LoadBuiltInLayout(crate::layout_update::BuiltInLayout::TopVolume24h),
        UpdateRoute::Layout,
    );
    assert_route(
        Message::LoadBuiltInLayout(crate::layout_update::BuiltInLayout::TopOpenInterest),
        UpdateRoute::Layout,
    );
    assert_route(
        Message::BuiltInLayoutContextsLoaded(
            1,
            crate::layout_update::BuiltInLayout::TopVolume24h,
            Ok(crate::api::WatchlistContextsResponse::complete(
                std::collections::HashMap::new(),
            )),
        ),
        UpdateRoute::Layout,
    );
    assert_route(Message::LayoutRenameToggled(0), UpdateRoute::Layout);
    assert_route(
        Message::LayoutRenameChanged("Main".to_string()),
        UpdateRoute::Layout,
    );
    assert_route(Message::LayoutRenameSubmitted(0), UpdateRoute::Layout);
}
