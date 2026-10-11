use super::*;

#[test]
fn builder_settings_messages_route_to_settings() {
    let request = crate::settings_state::BuilderApprovalRequest {
        generation: 1,
        account: "0x1111111111111111111111111111111111111111".into(),
        builder: crate::builder_codes::BuilderFee::new(
            "0x2222222222222222222222222222222222222222",
            10,
        )
        .expect("builder"),
        wallet: false,
    };
    for message in [
        Message::BuilderAddressChanged("test".to_string().into()),
        Message::BuilderFeeChanged("1".to_string().into()),
        Message::SaveBuilderCode,
        Message::SetBuilderCodeEnabled(true),
        Message::CheckBuilderApproval,
        Message::ApproveBuilderInWallet,
        Message::BuilderApprovalChecked(request.clone(), Ok(10)),
        Message::BuilderWalletApprovalFinished(request, Ok(())),
    ] {
        assert_route(message, UpdateRoute::Settings);
    }
}

#[test]
fn console_messages_route_to_console() {
    for message in [
        Message::OpenConsoleWindow,
        Message::ConsoleTick,
        Message::ConsoleTogglePause,
        Message::ConsoleProviderChanged(crate::network_activity::Provider::Hyperliquid),
        Message::ConsoleFilterChanged(crate::console_state::ConsoleFilter::Http),
        Message::ConsolePageChanged(1),
        Message::ConsoleClear,
    ] {
        assert_route(message, UpdateRoute::Console);
    }
}

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
    for enabled in [false, true] {
        assert_route(
            Message::TogglePaneDividers(enabled),
            UpdateRoute::Preferences,
        );
    }
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
        Message::OpenBuiltInLayoutInCanvas(crate::layout_update::BuiltInLayout::TopVolume24h),
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
