use super::*;

#[test]
fn market_chart_feed_and_export_routes_stay_on_their_feature_modules() {
    let source_context = crate::read_data_provider::MarketDataSourceContext {
        provider: crate::config::ReadDataProvider::Hyperliquid,
        read_data_provider_generation: 0,
        hydromancer_key_generation: None,
    };

    assert_route(
        Message::ClearDrawingTool(7, crate::chart_state::ChartSurfaceId::Docked(7)),
        UpdateRoute::Annotations,
    );
    assert_route(
        Message::HydromancerKeyInputChanged(String::new().into()),
        UpdateRoute::Feed,
    );
    assert_route(
        Message::XFeedAccessTokenChanged(String::new().into()),
        UpdateRoute::Feed,
    );
    assert_route(
        Message::XFeedOAuthClientIdChanged(String::new().into()),
        UpdateRoute::Feed,
    );
    assert_route(
        Message::XFeedRefreshTokenChanged(String::new().into()),
        UpdateRoute::Feed,
    );
    assert_route(Message::XFeedConnect, UpdateRoute::Feed);
    assert_route(
        Message::XAccessTokenRefreshed(
            1,
            crate::message::XAccessTokenRefreshMessageResult::new(Err(String::new())),
        ),
        UpdateRoute::Feed,
    );
    assert_route(Message::RefreshXFeed(0), UpdateRoute::Feed);
    assert_route(
        Message::XProfileImageLoaded(
            1,
            crate::message::XProfileImageMessageResult::new(Ok(Vec::new())),
        ),
        UpdateRoute::Feed,
    );
    assert_route(Message::RefreshHeatmap, UpdateRoute::Hyperdash);
    assert_route(
        Message::ReadDataProviderChanged(crate::config::ReadDataProvider::Hydromancer),
        UpdateRoute::Preferences,
    );
    assert_route(
        Message::ToggleHydromancerRealtimePositionPnl(true),
        UpdateRoute::Preferences,
    );
    assert_route(Message::OpenDetachedChart(7), UpdateRoute::Chart);
    assert_route(
        Message::ChartWsAssetCtxLagged(7, "BTC".to_string(), source_context, 9),
        UpdateRoute::Chart,
    );
    assert_route(
        Message::ChartAssetContextRestFetched(7, "xyz:NVDA".to_string(), Ok(None)),
        UpdateRoute::Chart,
    );
    assert_route(
        Message::ChartSpotAssetContextsRestFetched(vec![(7, "@107".to_string())], Ok(Vec::new())),
        UpdateRoute::Chart,
    );
    assert_route(
        Message::OpenChartEarningsFiling(7, crate::chart_state::ChartSurfaceId::Docked(7), 2_000),
        UpdateRoute::Chart,
    );
    assert_route(
        Message::ChartEarningsFilingSummaryLoaded(
            "1045810:0001045810-26-000051:nvda-20260520.htm".to_string(),
            1,
            Box::new(Err(String::new())),
        ),
        UpdateRoute::Chart,
    );
    assert_route(
        Message::ChartEarningsFilingOpenResult(Ok(())),
        UpdateRoute::Chart,
    );
    assert_route(
        Message::ChartHudControlChanged(
            7,
            crate::chart_state::ChartSurfaceId::Docked(7),
            crate::sound::HudUiSound::ModeMarket,
            true,
        ),
        UpdateRoute::Chart,
    );
    assert_route(
        Message::ToggleChartHudUiSounds(true),
        UpdateRoute::Preferences,
    );
    assert_route(
        Message::ToggleChartGradientBackground(true),
        UpdateRoute::Preferences,
    );
    assert_route(
        Message::OpenChartScreenshot(7, crate::chart_state::ChartSurfaceId::Docked(7)),
        UpdateRoute::ChartScreenshot,
    );
    assert_route(
        Message::ToggleChartScreenshotObscurePositionEntry(true),
        UpdateRoute::ChartScreenshot,
    );
    assert_route(
        Message::ToggleChartScreenshotHidePositionsAndOrders(true),
        UpdateRoute::ChartScreenshot,
    );
    assert_route(
        Message::OutcomeMarketGroupToggled("question:19".to_string()),
        UpdateRoute::Market,
    );
    assert_route(
        Message::OrderBookWsBookLagged {
            id: 7,
            coin: "BTC".to_string(),
            sigfigs: (Some(5), None),
            source_context,
            skipped: 9,
        },
        UpdateRoute::Market,
    );
    assert_route(
        Message::OrderBookWsAssetCtxLagged {
            id: 7,
            coin: "BTC".to_string(),
            source_context,
            skipped: 9,
        },
        UpdateRoute::Market,
    );
    assert_route(
        Message::PositioningInfoWsAssetCtxLagged("BTC".to_string(), source_context, 9),
        UpdateRoute::Market,
    );
    assert_route(
        Message::PositioningInfoEntryMinChanged(7, "20".to_string()),
        UpdateRoute::Market,
    );
    assert_route(
        Message::PositioningInfoEntryMaxChanged(7, "30".to_string()),
        UpdateRoute::Market,
    );
    assert_route(
        Message::ApplyPositioningInfoEntryRange(7),
        UpdateRoute::Market,
    );
    assert_route(Message::AddSessionDataPane, UpdateRoute::Market);
    assert_route(Message::RefreshSessionData(3), UpdateRoute::Market);
    assert_route(
        Message::SessionDataLookbackChanged(
            3,
            crate::session_data_state::SessionDataLookback::EightWeeks,
        ),
        UpdateRoute::Market,
    );
}
