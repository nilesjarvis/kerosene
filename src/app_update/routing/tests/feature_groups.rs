use super::*;

#[test]
fn alfred_screener_settings_and_window_routes_stay_on_feature_modules() {
    let window_id = window_id();

    assert_route(Message::ToggleAlfred, UpdateRoute::Alfred);
    assert_route(
        Message::AlfredSelectionMoved(crate::alfred_state::AlfredSelectionStep::Next),
        UpdateRoute::Alfred,
    );
    assert_route(
        Message::AlfredCommandSelected(crate::alfred_state::AlfredCommandId::OpenSettingsWindow),
        UpdateRoute::Alfred,
    );
    assert_route(Message::AddXFeedPane, UpdateRoute::Panes);
    assert_route(Message::AddPositionsHistoryPane, UpdateRoute::Panes);
    assert_route(
        Message::BeginWidgetPlacement(crate::pane_management::AddWidgetKind::OrderBook),
        UpdateRoute::Panes,
    );
    assert_route(Message::CancelWidgetPlacement, UpdateRoute::Panes);

    assert_route(Message::OpenScreenerWindow, UpdateRoute::Screener);
    assert_route(
        Message::ScreenerExchangeFilterChanged(
            crate::screener_state::ScreenerExchangeFilter::AllHip3,
        ),
        UpdateRoute::Screener,
    );
    assert_route(
        Message::ScreenerSortChanged(crate::screener_state::ScreenerSortColumn::Funding),
        UpdateRoute::Screener,
    );

    assert_route(Message::OpenSettingsWindow, UpdateRoute::Settings);
    assert_route(
        Message::SettingsTabSelected(crate::settings_state::SettingsTab::Storage),
        UpdateRoute::Settings,
    );
    assert_route(
        Message::ThemeSettingsPageSelected(crate::settings_state::ThemeSettingsPage::Fonts),
        UpdateRoute::Settings,
    );

    assert_route(Message::WindowDrag(window_id), UpdateRoute::Window);
    assert_route(
        Message::WindowResized(window_id, iced::Size::new(720.0, 480.0)),
        UpdateRoute::Window,
    );
    assert_route(Message::WindowClose(window_id), UpdateRoute::Window);
}

#[test]
fn openrouter_routes_stay_on_openrouter_module() {
    assert_route(
        Message::OpenRouterKeyInputChanged("sentinel-secret".into()),
        UpdateRoute::OpenRouter,
    );
    assert_route(Message::SaveOpenRouterKey, UpdateRoute::OpenRouter);
    assert_route(
        Message::OpenRouterKeyChecked(0, Err("key check failed".to_string())),
        UpdateRoute::OpenRouter,
    );
    assert_route(
        Message::OpenRouterModelChanged("openrouter/auto".to_string()),
        UpdateRoute::OpenRouter,
    );
}

#[test]
fn journal_spaghetti_and_wallet_tracker_routes_stay_on_feature_modules() {
    assert_route(Message::JournalRefresh, UpdateRoute::Journal);
    assert_route(
        Message::JournalFilterChanged(crate::journal::JournalFilter::Outcome),
        UpdateRoute::Journal,
    );
    assert_route(
        Message::JournalPortfolioWindowChanged(crate::portfolio_state::PortfolioWindow::Month),
        UpdateRoute::Journal,
    );
    assert_route(
        Message::JournalCauseOfErrorChanged("trade".to_string(), "late chase".to_string()),
        UpdateRoute::Journal,
    );

    assert_route(Message::AddComparisonChart, UpdateRoute::Spaghetti);
    assert_route(Message::ToggleSpaghettiStyleMenu(7), UpdateRoute::Spaghetti);
    assert_route(
        Message::SpaghettiSetColorMode(7, crate::spaghetti::ComparisonColorMode::Single),
        UpdateRoute::Spaghetti,
    );

    assert_route(Message::OpenWalletTrackerWindow, UpdateRoute::WalletTracker);
    assert_route(
        Message::OpenWalletClustersWindow,
        UpdateRoute::WalletCluster,
    );
    assert_route(Message::WalletClusterRefresh, UpdateRoute::WalletCluster);
    assert_route(
        Message::WalletClusterSubmitOrder { is_buy: true },
        UpdateRoute::WalletCluster,
    );
    assert_route(
        Message::OpenWalletDetailsWindow("0xabc".into()),
        UpdateRoute::WalletTracker,
    );
    assert_route(
        Message::WalletTrackerInputChanged("0xdef".into()),
        UpdateRoute::WalletTracker,
    );
    assert_route(Message::WalletTrackerRefreshDue, UpdateRoute::WalletTracker);
}
