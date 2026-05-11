use crate::message::Message;

// ---------------------------------------------------------------------------
// Update Routing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateRoute {
    Account,
    Annotations,
    Calendar,
    Chart,
    ChartScreenshot,
    Chrome,
    Feed,
    Hyperdash,
    Journal,
    Layout,
    Market,
    Order,
    PaneInteractions,
    Panes,
    PortfolioIncome,
    Preferences,
    Settings,
    Spaghetti,
    WalletTracker,
    Window,
}

pub(super) fn message_route(message: &Message) -> UpdateRoute {
    match message {
        Message::LayoutInputChanged(_)
        | Message::SaveLayout(_)
        | Message::LoadLayout(_)
        | Message::DeleteLayout(_)
        | Message::ExportLayout(_)
        | Message::ImportLayout
        | Message::ExportWalletLabels
        | Message::ImportWalletLabels
        | Message::LayoutExported(_)
        | Message::LayoutImported(_)
        | Message::WalletLabelsExported(_)
        | Message::WalletLabelsImported(_) => UpdateRoute::Layout,

        Message::PaneResized(_)
        | Message::PaneDragged(_)
        | Message::PaneClicked(_)
        | Message::ClosePane(_) => UpdateRoute::PaneInteractions,

        Message::SwitchBottomTab(_)
        | Message::CloseAllMenus
        | Message::ToggleAddWidgetMenu
        | Message::SetAddWidgetPlacement(_)
        | Message::AddPortfolioPane
        | Message::AddIncomePane
        | Message::AddTradingJournal
        | Message::AddCalendarPane
        | Message::AddLiquidationsPane
        | Message::AddAdvancedOrdersPane
        | Message::AddTrackedTradesPane
        | Message::AddOutcomesPane => UpdateRoute::Panes,

        Message::ToggleHidePnl
        | Message::ToggleIncomeAlerts
        | Message::ToggleLiquidationAlerts
        | Message::ToggleTrackedTradeAlerts
        | Message::ToggleTrackedTradeAggregation
        | Message::ToggleLiquidationFeedAggregation
        | Message::ToggleLiquidationChart
        | Message::ToggleLiquidationSummary
        | Message::LiquidationAlertThresholdChanged(_)
        | Message::SaveLiquidationAlertThreshold
        | Message::DismissToast(_)
        | Message::CopyToClipboard(_)
        | Message::NoOp
        | Message::TickToastCleanup
        | Message::SpinnerTick
        | Message::StatusBarTick
        | Message::ConfigSaved(_)
        | Message::CalendarImpactFilterChanged(_)
        | Message::CalendarWindowFilterChanged(_)
        | Message::ToggleSound
        | Message::TestSound
        | Message::ToggleDesktopNotifications => UpdateRoute::Chrome,

        Message::OrderPriceChanged(_)
        | Message::SetMidPrice
        | Message::OrderQuantityChanged(_)
        | Message::ToggleOrderDenomination
        | Message::OrderPercentageChanged(_)
        | Message::SetOrderKind(_)
        | Message::ToggleReduceOnly
        | Message::TogglePresetsMenu
        | Message::TogglePresetCurrency
        | Message::TogglePresetEditMode
        | Message::EditPresetStart(_, _, _)
        | Message::EditPresetChanged(_)
        | Message::EditPresetSave(_, _)
        | Message::ExecutePreset(_, _, _)
        | Message::DismissOrderStatus
        | Message::PlaceBuy
        | Message::PlaceSell
        | Message::OrderResult(_)
        | Message::CancelOrder { .. }
        | Message::CancelResult(_)
        | Message::ToggleCloseMenu(_)
        | Message::ClosePosition { .. }
        | Message::ClosePositionResult(_)
        | Message::NukePositions
        | Message::NukeResult(_)
        | Message::StartChase(_)
        | Message::StopChase
        | Message::StopChaseById(_)
        | Message::StopAllAdvancedOrders
        | Message::TwapDurationChanged(_)
        | Message::TwapSlicesChanged(_)
        | Message::TwapMinPriceChanged(_)
        | Message::TwapMaxPriceChanged(_)
        | Message::TwapRandomizeToggled(_)
        | Message::StartTwap(_)
        | Message::StopTwap(_)
        | Message::TwapTick
        | Message::TwapBookUpdate { .. }
        | Message::TwapSliceResult { .. }
        | Message::TwapUnexpectedCancelResult { .. }
        | Message::OpenTwapDetails(_)
        | Message::OpenAdvancedOrderHistory(_)
        | Message::ChaseInitialBookLoaded { .. }
        | Message::ChaseBookUpdate { .. }
        | Message::ChaseRepriceTick
        | Message::ChasePlaceResult { .. }
        | Message::ChaseModifyResult { .. }
        | Message::ChaseCancelResult { .. }
        | Message::OpenQuickOrder(_, _, _, _, _, _)
        | Message::QuickOrderQtyChanged(_, _)
        | Message::QuickOrderToggleType(_)
        | Message::CloseQuickOrder(_)
        | Message::SubmitQuickOrder(_, _)
        | Message::QuickOrderResult(_)
        | Message::EscapePressed
        | Message::MoveOrder { .. }
        | Message::MoveOrderModifyResult { .. }
        | Message::ChaseRestingOrder { .. } => UpdateRoute::Order,

        Message::ToggleFavourite(_)
        | Message::SymbolsLoaded(_)
        | Message::LiveWatchlistSortChanged(_, _)
        | Message::LiveWatchlistColumnToggled(_, _, _)
        | Message::AddOrderBookPane
        | Message::AddLiveWatchlistPane
        | Message::LiveWatchlistSearchChanged(_, _)
        | Message::LiveWatchlistAddSymbol(_, _)
        | Message::LiveWatchlistRemoveSymbol(_, _)
        | Message::LiveWatchlistRefreshTick
        | Message::LiveWatchlistContextsLoaded(_, _)
        | Message::LiveWatchlistHistoryLoaded(_, _, _)
        | Message::SymbolSearchChanged(_)
        | Message::SymbolSearchSortChanged(_)
        | Message::SymbolSearchMarketFilterChanged(_)
        | Message::SymbolSearchHip3DexFilterChanged(_)
        | Message::SymbolSearchContextsLoaded(_, _)
        | Message::SymbolSelected(_)
        | Message::BookLoaded(_, _)
        | Message::OrderBookWsAssetCtxUpdate(_, _)
        | Message::WsBookUpdate(_, _, _)
        | Message::SetBookTickSize(_, _)
        | Message::ToggleOrderBookSettings(_)
        | Message::ToggleOrderBookSpreadChart(_)
        | Message::OrderBookSpreadChartResize(_, _)
        | Message::OrderBookSearchChanged(_, _)
        | Message::OrderBookSetMode(_, _)
        | Message::SetOrderBookDisplayMode(_, _)
        | Message::CenterOrderBook(_) => UpdateRoute::Market,

        Message::ThemeChanged(_)
        | Message::MutedTickerInputChanged(_)
        | Message::MuteTicker
        | Message::UnmuteTicker(_)
        | Message::MarketSlippageInputChanged(_)
        | Message::SaveMarketSlippage
        | Message::StartRecordingHotkey(_)
        | Message::KeyboardEvent(_, _)
        | Message::ExecuteHotkey(_) => UpdateRoute::Preferences,

        Message::OpenSettingsWindow
        | Message::SettingsTabSelected(_)
        | Message::OpenUnlockCredentialsPopup
        | Message::DismissUnlockCredentialsPopup
        | Message::OpenCredentialStorageSettings
        | Message::SecretStorageSelectionChanged(_)
        | Message::EncryptedSecretPasswordChanged(_)
        | Message::EncryptedSecretConfirmChanged(_)
        | Message::UnlockEncryptedSecrets
        | Message::ApplySecretStorageSelection
        | Message::ClearConfigs
        | Message::ConfigsCleared(_) => UpdateRoute::Settings,

        Message::RefreshCalendar | Message::CalendarLoaded(_) | Message::Tick => {
            UpdateRoute::Calendar
        }

        Message::WindowMoved(_, _)
        | Message::WindowOpened(_)
        | Message::WindowClosed(_)
        | Message::WindowResized(_, _) => UpdateRoute::Window,

        Message::OpenChartScreenshot(_)
        | Message::ChartScreenshotBoundsResolved(_, _)
        | Message::ChartScreenshotCaptured(_, _)
        | Message::CopyChartScreenshot
        | Message::ChartScreenshotCopied(_)
        | Message::SaveChartScreenshot
        | Message::ChartScreenshotSaved(_)
        | Message::CloseChartScreenshotWindow => UpdateRoute::ChartScreenshot,

        Message::JournalFillsLoaded { .. }
        | Message::JournalEditStart(_, _)
        | Message::JournalEditCancel(_)
        | Message::JournalEditSave(_)
        | Message::JournalBufferChanged(_, _, _)
        | Message::JournalFilterChanged(_)
        | Message::JournalSortChanged(_)
        | Message::JournalToggleAllAssets
        | Message::JournalRefresh => UpdateRoute::Journal,

        Message::AddComparisonChart
        | Message::AddPairRatioChart
        | Message::SpaghettiReload(_)
        | Message::SpaghettiSwitchTimeframe(_, _)
        | Message::SpaghettiCandlesLoaded(_, _)
        | Message::SpaghettiWsCandleUpdate(_, _, _)
        | Message::SpaghettiOpenEditor(_)
        | Message::SpaghettiCloseEditor(_)
        | Message::SpaghettiEditorSearchChanged(_, _)
        | Message::SpaghettiAddSymbol(_, _)
        | Message::SpaghettiRemoveSymbol(_, _)
        | Message::SpaghettiSetSession(_, _)
        | Message::SpaghettiSetSessionGranularityAuto(_)
        | Message::SpaghettiResetView(_)
        | Message::ToggleSpaghettiStyleMenu(_)
        | Message::ToggleSpaghettiLabels(_)
        | Message::SpaghettiSetColorMode(_, _)
        | Message::PairSetCandleMode(_, _) => UpdateRoute::Spaghetti,

        Message::OpenWalletTrackerWindow
        | Message::OpenWalletDetailsWindow(_)
        | Message::RefreshWalletDetails(_)
        | Message::WalletDetailsLoaded(_, _, _)
        | Message::WalletDetailsWsUpdate(_, _)
        | Message::WalletTrackerInputChanged(_)
        | Message::WalletTrackerLabelInputChanged(_)
        | Message::WalletTrackerAdd
        | Message::WalletTrackerRemove(_)
        | Message::WalletTrackerLabelChanged(_, _)
        | Message::WalletTrackerRefresh
        | Message::WalletTrackerRefreshDue
        | Message::WalletTrackerRefreshOne(_)
        | Message::WalletTrackerRefreshOrdersDue
        | Message::WalletTrackerRefreshOrders(_)
        | Message::WalletTrackerLoaded(_, _)
        | Message::WalletTrackerOrdersLoaded(_, _) => UpdateRoute::WalletTracker,

        Message::RefreshPortfolio
        | Message::PortfolioLoaded(_, _)
        | Message::RefreshIncome
        | Message::IncomeLoaded(_, _)
        | Message::SetPortfolioScope(_)
        | Message::SetPortfolioWindow(_) => UpdateRoute::PortfolioIncome,

        Message::SetDrawingTool(_, _)
        | Message::AddAnnotation(_)
        | Message::RemoveAnnotation(_)
        | Message::ClearDrawingTool => UpdateRoute::Annotations,

        Message::ChartReload(_)
        | Message::ChartResetView(_)
        | Message::ChartSwitchTimeframe(_, _)
        | Message::ToggleMacroMenu(_)
        | Message::ToggleMacroIndicator(_, _)
        | Message::MacroCandlesLoaded(_, _, _, _)
        | Message::ChartCandlesLoaded(_, _)
        | Message::ChartFundingHistoryLoaded(_, _)
        | Message::ChartWsCandleUpdate(_, _, _, _)
        | Message::ChartWsAssetCtxUpdate(_, _, _)
        | Message::ChartViewportChanged(_, _)
        | Message::ChartFundingPanelHeightChanged(_, _, _)
        | Message::ToggleFundingRateDisplayMode(_)
        | Message::FundingRefreshTick
        | Message::ToggleOpenInterestNotional(_)
        | Message::ChartSymbolSelected(_, _)
        | Message::ToggleChartInvert(_)
        | Message::ToggleChartTradeMarkers(_)
        | Message::ChartOpenEditor(_)
        | Message::ChartCloseEditor(_)
        | Message::ChartEditorSearchChanged(_, _)
        | Message::ChartEditorSubmit(_)
        | Message::AddChart(_) => UpdateRoute::Chart,

        Message::PositionsSortChanged(_)
        | Message::ToggleHiddenPosition(_)
        | Message::ToggleShowHiddenPositions
        | Message::WalletKeyInputChanged(_)
        | Message::WalletAddressInputChanged(_)
        | Message::AccountLabelChanged(_)
        | Message::ToggleAccountPicker
        | Message::AccountPickerSelected(_)
        | Message::AddAccount
        | Message::GhostWallet(_)
        | Message::ForgetGhostAccount(_)
        | Message::DeleteSavedAccount(_)
        | Message::SaveCredentials
        | Message::ConnectWallet
        | Message::DisconnectWallet
        | Message::AccountDataLoaded(_, _)
        | Message::RefreshAccountData
        | Message::AllMidsBootstrapLoaded(_, _)
        | Message::WsUserDataUpdate(_, _) => UpdateRoute::Account,

        Message::HydromancerKeyInputChanged(_)
        | Message::SaveHydromancerKey
        | Message::ReconnectLiquidations
        | Message::ReconnectTrackedTrades
        | Message::WsHydromancerLiquidation(_)
        | Message::WsHydromancerTrackedTrades(_)
        | Message::ClearLiquidations
        | Message::ClearTrackedTrades => UpdateRoute::Feed,

        Message::HyperdashKeyInputChanged(_)
        | Message::SaveHyperdashKey
        | Message::ToggleLiquidationOverlay(_)
        | Message::ChartLiquidationLoaded(_, _)
        | Message::RefreshLiquidations
        | Message::ToggleHeatmapOverlay(_)
        | Message::ChartHeatmapLoaded(_, _)
        | Message::RefreshHeatmap => UpdateRoute::Hyperdash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_messages_with_known_overlap_to_existing_update_modules() {
        assert_eq!(message_route(&Message::Tick), UpdateRoute::Calendar);
        assert_eq!(
            message_route(&Message::CalendarImpactFilterChanged(
                crate::calendar_state::CalendarImpactFilter::All,
            )),
            UpdateRoute::Chrome
        );
        assert_eq!(message_route(&Message::ToggleHidePnl), UpdateRoute::Chrome);
        assert_eq!(
            message_route(&Message::ToggleHiddenPosition("BTC".to_string())),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::DismissOrderStatus),
            UpdateRoute::Order
        );
        assert_eq!(
            message_route(&Message::RefreshPortfolio),
            UpdateRoute::PortfolioIncome
        );
        assert_eq!(
            message_route(&Message::ClearDrawingTool),
            UpdateRoute::Annotations
        );
        assert_eq!(
            message_route(&Message::ConfigSaved(Ok(()))),
            UpdateRoute::Chrome
        );
        assert_eq!(
            message_route(&Message::RefreshAccountData),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::HydromancerKeyInputChanged(String::new())),
            UpdateRoute::Feed
        );
        assert_eq!(
            message_route(&Message::RefreshHeatmap),
            UpdateRoute::Hyperdash
        );
    }
}
