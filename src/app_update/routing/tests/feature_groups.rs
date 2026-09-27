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
    assert_route(Message::CreateCanvas, UpdateRoute::Canvas);
    assert_route(Message::OpenCanvas(7), UpdateRoute::Canvas);

    assert_route(Message::OpenScreenerWindow, UpdateRoute::Screener);
    assert_route(
        Message::LiveWatchlistPresetSelected(3, 9),
        UpdateRoute::Market,
    );
    assert_route(
        Message::WatchlistPresetNameChanged(9, "Majors".to_string()),
        UpdateRoute::Market,
    );
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
fn assistant_routes_stay_on_agent_module() {
    assert_route(Message::OpenAgentWindow, UpdateRoute::Agent);
    assert_route(
        Message::AgentInputChanged("private question".to_string().into()),
        UpdateRoute::Agent,
    );
    assert_route(Message::AgentSubmit, UpdateRoute::Agent);
    assert_route(Message::AgentPnlCardBrowse, UpdateRoute::Agent);
    assert_route(
        Message::AgentPnlCardDropped(
            window_id(),
            crate::agent_pnl_card::AgentPnlCardPath::new("card.png".into()),
        ),
        UpdateRoute::Agent,
    );
    assert_route(
        Message::AgentPnlCardHoverChanged(window_id(), true),
        UpdateRoute::Agent,
    );
    assert_route(
        Message::AgentPnlCardLoaded(1, Ok(None).into()),
        UpdateRoute::Agent,
    );
    assert_route(Message::AgentPnlCardRemove, UpdateRoute::Agent);
    assert_route(Message::AgentAbort, UpdateRoute::Agent);
    assert_route(Message::AgentStreamTick, UpdateRoute::Agent);
    assert_route(Message::AgentCopyResponse(4), UpdateRoute::Agent);
    assert_route(Message::AgentRegenerateResponse(4), UpdateRoute::Agent);
    assert_route(Message::AgentToggleToolTrace(4), UpdateRoute::Agent);
    assert_route(Message::AgentToggleReasoning(4), UpdateRoute::Agent);
    assert_route(
        Message::AgentFollowUpSelected("Compare the sessions".to_string().into()),
        UpdateRoute::Agent,
    );
    assert_route(Message::AgentNewChat, UpdateRoute::Agent);
    assert_route(Message::AgentSelectSession(42), UpdateRoute::Agent);
    assert_route(Message::AgentToggleSidebar, UpdateRoute::Agent);
    assert_route(
        Message::AgentProviderChanged(crate::config::AssistantProvider::LlamaCpp),
        UpdateRoute::Agent,
    );
    assert_route(
        Message::AgentLocalServerDetected(3, Ok(None)),
        UpdateRoute::Agent,
    );
    assert_route(Message::AgentToggleModelPicker, UpdateRoute::Agent);
    assert_route(
        Message::AgentModelSearchChanged("claude".to_string()),
        UpdateRoute::Agent,
    );
    assert_route(Message::AgentRefreshModels, UpdateRoute::Agent);
    assert_route(
        Message::AgentModelCatalogLoaded(7, Ok(Vec::new())),
        UpdateRoute::Agent,
    );
    assert_route(
        Message::AgentSessionsSaved(42, Ok(()).into()),
        UpdateRoute::Agent,
    );
    assert_route(
        Message::AgentOpenLink("https://example.com".to_string().into()),
        UpdateRoute::Agent,
    );
    assert_route(Message::AgentLinkOpened(Ok(())), UpdateRoute::Agent);
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
    assert_route(
        Message::SpaghettiWatchlistPresetSelected(7, 12),
        UpdateRoute::Spaghetti,
    );
    assert_route(
        Message::SpaghettiClearWatchlistPreset(7),
        UpdateRoute::Spaghetti,
    );
    assert_route(Message::ToggleSpaghettiStyleMenu(7), UpdateRoute::Spaghetti);
    assert_route(
        Message::SpaghettiSetColorMode(7, crate::spaghetti::ComparisonColorMode::Single),
        UpdateRoute::Spaghetti,
    );
    assert_route(
        Message::OpenDetachedSpaghettiChart(7),
        UpdateRoute::Spaghetti,
    );

    assert_route(Message::OpenWalletTrackerWindow, UpdateRoute::WalletTracker);
    assert_route(
        Message::OpenCombinedPortfolioWindow,
        UpdateRoute::CombinedPortfolio,
    );
    assert_route(
        Message::CombinedPortfolioWindowChanged(crate::portfolio_state::PortfolioWindow::AllTime),
        UpdateRoute::CombinedPortfolio,
    );
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

#[test]
fn hyperliquid_proxy_settings_route_to_settings() {
    for message in [
        Message::HyperliquidProxyInputChanged("http://sentinel:secret@proxy.test".into()),
        Message::AddHyperliquidProxy,
        Message::RemoveHyperliquidProxy(2),
        Message::SetHyperliquidProxiesEnabled(true),
        Message::SettingsTabSelected(crate::settings_state::SettingsTab::Network),
    ] {
        assert!(!format!("{message:?}").contains("sentinel"));
        assert_route(message, UpdateRoute::Settings);
    }
}

#[test]
fn compact_wallet_routes_reach_panes_and_wallet_tracker() {
    assert_route(Message::AddCompactWalletTrackerPane, UpdateRoute::Panes);
    for message in [
        Message::CompactWalletSelected(7, "0xabc0000000000000000000000000000000000000".into()),
        Message::CompactWalletBack(7),
        Message::CompactWalletRefresh(7),
        Message::CompactWalletDetailsLoaded(
            7,
            1,
            crate::read_data_provider::ReadDataRequestContext {
                provider: crate::config::ReadDataProvider::Hyperliquid,
                read_data_provider_generation: 0,
                hydromancer_key_generation: 0,
            },
            crate::wallet_state::compact::CompactWalletDetailsResult(Err("failed".into())),
        ),
    ] {
        assert_route(message, UpdateRoute::WalletTracker);
    }
}

#[test]
fn remote_wallet_database_messages_route_to_wallet_tracker() {
    for message in [
        Message::RemoteWalletDatabaseUrlChanged("http://localhost:8090".into()),
        Message::SaveRemoteWalletDatabase,
        Message::DisconnectRemoteWalletDatabase,
        Message::RemoteWalletDatabaseSync,
        Message::RemoteWalletDatabaseLoaded(
            7,
            crate::wallet_state::remote_database::RemoteWalletDatabaseResult(
                Ok(Default::default()),
            ),
        ),
    ] {
        assert_route(message, UpdateRoute::WalletTracker);
    }
}
