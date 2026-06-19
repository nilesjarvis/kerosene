use crate::message::Message;

// ---------------------------------------------------------------------------
// Update Routing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateRoute {
    Account,
    Alfred,
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
    Screener,
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
        | Message::UpdateActiveLayout
        | Message::LayoutRenameToggled(_)
        | Message::LayoutRenameChanged(_)
        | Message::LayoutRenameSubmitted(_)
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
        | Message::ToggleLayoutMenu
        | Message::ToggleTickerTape
        | Message::SetAddWidgetPlacement(_)
        | Message::AddPortfolioPane
        | Message::AddIncomePane
        | Message::AddTradingJournal
        | Message::AddCalendarPane
        | Message::AddLiquidationsPane
        | Message::AddLiquidationsDistributionPane
        | Message::AddAdvancedOrdersPane
        | Message::AddTrackedTradesPane
        | Message::AddTelegramFeedPane
        | Message::AddOutcomesPane
        | Message::AddHypeEtfsPane
        | Message::AddHypeUnstakingQueuePane => UpdateRoute::Panes,

        Message::ToggleHidePnl
        | Message::TickerTapeTick
        | Message::ToggleIncomeAlerts
        | Message::ToggleLiquidationAlerts
        | Message::ToggleTrackedTradeAlerts
        | Message::ToggleTrackedTradeAggregation
        | Message::ToggleTrackedTradeSettingsMenu
        | Message::ToggleLiquidationFeedAggregation
        | Message::ToggleLiquidationChart
        | Message::ToggleLiquidationSummary
        | Message::ToggleLiquidationFollow
        | Message::ToggleLiquidationSettingsMenu
        | Message::LiquidationAlertThresholdChanged(_)
        | Message::SaveLiquidationAlertThreshold
        | Message::DismissToast(_)
        | Message::ToastAnimationTick
        | Message::CopyToClipboard(_)
        | Message::WalletAddressActionsHovered(_)
        | Message::WalletAddressActionsExited(_)
        | Message::NoOp
        | Message::TickToastCleanup
        | Message::SpinnerTick
        | Message::StatusBarTick
        | Message::ConfigSaved(_)
        | Message::CalendarImpactFilterChanged(_)
        | Message::CalendarWindowFilterChanged(_)
        | Message::ToggleSound
        | Message::ToggleDesktopNotifications => UpdateRoute::Chrome,

        Message::OrderPriceChanged(_)
        | Message::SetMidPrice
        | Message::OrderBookPriceSelected { .. }
        | Message::OrderQuantityChanged(_)
        | Message::ToggleOrderDenomination
        | Message::OrderPercentageChanged(_)
        | Message::PrefillOutcomeSell(_)
        | Message::SetOrderKind(_)
        | Message::ToggleReduceOnly
        | Message::ToggleOrderLeverageDropdown
        | Message::OrderLeverageInputChanged(_)
        | Message::SetOrderLeverageCross(_)
        | Message::SubmitOrderLeverage(_)
        | Message::OrderLeverageResult { .. }
        | Message::TogglePresetsMenu
        | Message::TogglePresetCurrency
        | Message::TogglePresetEditMode
        | Message::EditPresetStart(_, _, _)
        | Message::EditPresetChanged(_)
        | Message::EditPresetSave(_, _)
        | Message::ExecutePreset(_, _, _)
        | Message::DismissOrderStatus
        | Message::PlaceOrder { .. }
        | Message::OrderResult { .. }
        | Message::CancelOrder { .. }
        | Message::CancelResult { .. }
        | Message::CancelOrderStatusLoaded { .. }
        | Message::ToggleCloseMenu(_)
        | Message::ClosePosition { .. }
        | Message::ClosePositionResult { .. }
        | Message::NukePositions
        | Message::NukeResult { .. }
        | Message::NukePlacementStatusLoaded { .. }
        | Message::OneShotPlacementStatusLoaded { .. }
        | Message::StartChase { .. }
        | Message::StopChase
        | Message::StopChaseById(_)
        | Message::StopAllAdvancedOrders
        | Message::TwapDurationChanged(_)
        | Message::TwapSlicesChanged(_)
        | Message::TwapMinPriceChanged(_)
        | Message::TwapMaxPriceChanged(_)
        | Message::TwapRandomizeToggled(_)
        | Message::StartTwap { .. }
        | Message::StopTwap(_)
        | Message::TwapTick
        | Message::TwapBookUpdate { .. }
        | Message::TwapBookLagged { .. }
        | Message::TwapSliceResult { .. }
        | Message::TwapUnexpectedCancelResult { .. }
        | Message::TwapUnexpectedCancelRetryDue { .. }
        | Message::TwapOrderStatusLoaded { .. }
        | Message::OpenTwapDetails(_)
        | Message::OpenAdvancedOrderHistory(_)
        | Message::ChaseInitialBookLoaded { .. }
        | Message::ChaseBookUpdate { .. }
        | Message::ChaseBookLagged { .. }
        | Message::ChaseRepriceTick
        | Message::ChasePlaceResult { .. }
        | Message::ChaseModifyResult { .. }
        | Message::ChaseCancelResult { .. }
        | Message::ChaseOrderStatusLoaded { .. }
        | Message::ChaseOrderOidStatusLoaded { .. }
        | Message::OpenQuickOrder(_, _, _, _, _, _, _)
        | Message::QuickOrderQtyChanged(_, _)
        | Message::QuickOrderPercentageChanged(_, _)
        | Message::QuickOrderToggleDenomination(_)
        | Message::QuickOrderToggleType(_)
        | Message::CloseQuickOrder(_)
        | Message::SubmitQuickOrder { .. }
        | Message::QuickOrderResult { .. }
        | Message::SubmitHudOrder(_)
        | Message::HudOrderResult { .. }
        | Message::EscapePressed(_)
        | Message::MoveOrderDragStarted { .. }
        | Message::MoveOrder { .. }
        | Message::MoveOrderModifyResult { .. }
        | Message::MoveOrderStatusLoaded { .. }
        | Message::ChaseRestingOrder { .. } => UpdateRoute::Order,

        Message::ToggleFavourite(_)
        | Message::TickerTapeRefreshTick
        | Message::TickerTapeContextsLoaded(_, _, _, _)
        | Message::SymbolsLoaded(_)
        | Message::ExchangeSymbolsRefreshTick
        | Message::LiveWatchlistSortChanged(_, _)
        | Message::LiveWatchlistColumnToggled(_, _, _)
        | Message::ToggleLiveWatchlistSettings(_)
        | Message::AddOrderBookPane
        | Message::AddLiveWatchlistPane
        | Message::AddPositioningInfoPane
        | Message::PositioningInfoPageChanged(_, _)
        | Message::PositioningInfoSearchChanged(_, _)
        | Message::TogglePositioningInfoSymbolPicker(_)
        | Message::PositioningInfoSymbolSelected(_, _)
        | Message::PositioningInfoSideChanged(_, _)
        | Message::PositioningInfoSortChanged(_, _)
        | Message::PositioningInfoChangeTimeframeChanged(_, _)
        | Message::ClearPositioningInfoFilters(_)
        | Message::RefreshPositioningInfoPane(_)
        | Message::RefreshPositioningInfo
        | Message::PositioningInfoWsAssetCtxUpdate(_, _, _)
        | Message::PositioningInfoWsAssetCtxLagged(_, _, _)
        | Message::PositioningInfoLoaded(_, _, _)
        | Message::PositioningInfoChangeLoaded(_, _, _)
        | Message::LiveWatchlistSearchChanged(_, _)
        | Message::LiveWatchlistAddSymbol(_, _)
        | Message::LiveWatchlistRemoveSymbol(_, _)
        | Message::LiveWatchlistRefreshTick
        | Message::LiveWatchlistContextsLoaded(_, _, _, _)
        | Message::LiveWatchlistHistoryLoaded(_, _, _, _)
        | Message::SymbolSearchChanged(_)
        | Message::SymbolSearchSortChanged(_)
        | Message::SymbolSearchMarketFilterChanged(_)
        | Message::SymbolSearchHip3DexFilterChanged(_)
        | Message::SymbolSearchContextsLoaded(_, _, _, _)
        | Message::OutcomeSearchChanged(_)
        | Message::OutcomeMarketGroupToggled(_)
        | Message::OutcomeVolumesLoaded(_, _, _)
        | Message::RefreshHypeEtfs
        | Message::HypeEtfsRefreshTick
        | Message::HypeEtfsViewChanged(_)
        | Message::HypeEtfsLoaded(_, _)
        | Message::RefreshHypeUnstakingQueue
        | Message::HypeUnstakingQueueRefreshTick
        | Message::HypeUnstakingWindowChanged(_)
        | Message::HypeUnstakingAmountFilterChanged(_)
        | Message::HypeUnstakingSortChanged(_)
        | Message::ToggleHypeUnstakingMineOnly
        | Message::ClearHypeUnstakingFilters
        | Message::HypeUnstakingQueueLoaded(_, _)
        | Message::AddSessionDataPane
        | Message::SessionDataSearchChanged(_, _)
        | Message::ToggleSessionDataSymbolPicker(_)
        | Message::SessionDataSymbolSelected(_, _)
        | Message::SessionDataLookbackChanged(_, _)
        | Message::RefreshSessionData(_)
        | Message::SessionDataCandlesLoaded(_, _)
        | Message::SymbolSelected(_)
        | Message::BookLoaded { .. }
        | Message::OrderBookWsAssetCtxUpdate { .. }
        | Message::OrderBookWsAssetCtxLagged { .. }
        | Message::WsBookUpdate { .. }
        | Message::OrderBookWsBookLagged { .. }
        | Message::SetBookTickSize(_, _)
        | Message::ToggleOrderBookSettings(_)
        | Message::ToggleOrderBookCenterOnMid(_)
        | Message::ToggleOrderBookReverseSide(_)
        | Message::ToggleOrderBookSpreadChart(_)
        | Message::OrderBookSpreadChartResize(_, _)
        | Message::OrderBookSearchChanged(_, _)
        | Message::OrderBookSetMode(_, _)
        | Message::SetOrderBookDisplayMode(_, _) => UpdateRoute::Market,

        Message::ThemeChanged(_)
        | Message::UiScaleChanged(_)
        | Message::ToggleChartDottedBackground(_)
        | Message::ChartDottedBackgroundOpacityChanged(_)
        | Message::ChartHollowCandleModeChanged(_)
        | Message::ChartSeriesStyleChanged(_)
        | Message::JournalTradesViewChanged(_)
        | Message::ToggleChartFisheye(_)
        | Message::ChartFisheyeStrengthChanged(_)
        | Message::ToggleChartChromaticAberration(_)
        | Message::ChartChromaticAberrationStrengthChanged(_)
        | Message::ToggleChartEdgeBlur(_)
        | Message::ChartEdgeBlurStrengthChanged(_)
        | Message::ChartCrosshairStyleChanged(_)
        | Message::ToggleChartCrosshairGuides(_)
        | Message::ChartCrosshairScaleChanged(_)
        | Message::ToastPositionChanged(_)
        | Message::ToggleToastAnimations(_)
        | Message::ChartHudReadoutToggled(_, _)
        | Message::ChartHudOrderSoundChanged(_)
        | Message::ChartHudOrderSoundVolumeChanged(_)
        | Message::ImportChartHudOrderSound
        | Message::ChartHudOrderSoundImported(_)
        | Message::TestChartHudOrderSound
        | Message::ToggleChartHudUiSounds(_)
        | Message::ReadDataProviderChanged(_)
        | Message::AlfredPopupScaleChanged(_)
        | Message::DisplayFontChanged(_)
        | Message::MonospaceFontChanged(_)
        | Message::ImportDisplayFont
        | Message::DisplayFontImported(_)
        | Message::ImportMonospaceFont
        | Message::MonospaceFontImported(_)
        | Message::PaneBorderThicknessChanged(_)
        | Message::PaneCornerRadiusChanged(_)
        | Message::ToggleOuterWidgetBorder(_)
        | Message::DefaultWidgetPaddingChanged(_)
        | Message::FocusedWidgetPaddingChanged(_)
        | Message::ResetFocusedWidgetPadding
        | Message::ToggleCustomWindowChrome(_)
        | Message::MutedTickerInputChanged(_)
        | Message::MuteTicker
        | Message::UnmuteTicker(_)
        | Message::MarketUniverseChanged(_)
        | Message::DisplayDenominationChanged(_)
        | Message::MarketSlippageInputChanged(_)
        | Message::SaveMarketSlippage
        | Message::ToggleOptimisticAccountUpdates(_)
        | Message::StartRecordingHotkey(_)
        | Message::ClearHotkey(_)
        | Message::KeyboardEvent(_, _, _)
        | Message::ExecuteHotkey(_) => UpdateRoute::Preferences,

        Message::ToggleAlfred
        | Message::CloseAlfred
        | Message::AlfredQueryChanged(_)
        | Message::AlfredSelectionMoved(_)
        | Message::AlfredSubmit
        | Message::AlfredCommandSelected(_) => UpdateRoute::Alfred,

        Message::OpenScreenerWindow
        | Message::RefreshScreener
        | Message::ForceRefreshScreener
        | Message::RefreshScreenerHistory
        | Message::ScreenerExchangeFilterChanged(_)
        | Message::ScreenerSortChanged(_)
        | Message::ScreenerContextsLoaded(_, _, _, _)
        | Message::ScreenerHistoryLoaded(_, _, _, _) => UpdateRoute::Screener,

        Message::OpenSettingsWindow
        | Message::SettingsTabSelected(_)
        | Message::ThemeSettingsPageSelected(_)
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

        Message::RefreshCalendar | Message::CalendarLoaded(_, _) | Message::Tick => {
            UpdateRoute::Calendar
        }

        Message::WindowMoved(_, _)
        | Message::WindowOpened(_)
        | Message::WindowClosed(_)
        | Message::WindowResized(_, _)
        | Message::WindowDrag(_)
        | Message::WindowDragResize(_, _)
        | Message::WindowMinimize(_)
        | Message::WindowToggleMaximize(_)
        | Message::WindowClose(_) => UpdateRoute::Window,

        Message::ToggleChartScreenshotMenu(_, _)
        | Message::ToggleChartScreenshotObscurePositionEntry(_)
        | Message::ToggleChartScreenshotHidePositionsAndOrders(_)
        | Message::OpenChartScreenshot(_, _)
        | Message::ChartScreenshotBoundsResolved(_, _, _, _)
        | Message::ChartScreenshotCaptured(_, _, _)
        | Message::CopyChartScreenshot
        | Message::ChartScreenshotCopied(_)
        | Message::SaveChartScreenshot
        | Message::ChartScreenshotSaved(_)
        | Message::CloseChartScreenshotWindow => UpdateRoute::ChartScreenshot,

        Message::JournalFillsLoaded { .. }
        | Message::JournalClearCache
        | Message::JournalEditStart(_, _)
        | Message::JournalEditCancel(_)
        | Message::JournalEditSave(_)
        | Message::JournalBufferChanged(_, _, _)
        | Message::JournalFilterChanged(_)
        | Message::JournalSortChanged(_)
        | Message::JournalPortfolioWindowChanged(_)
        | Message::JournalChartRevealTick
        | Message::JournalToggleAllAssets
        | Message::JournalToggleAccountValueChart(_)
        | Message::JournalToggleIncludeFeesInPnl
        | Message::JournalSnapshotToggle(_)
        | Message::JournalSnapshotLoaded { .. }
        | Message::JournalRefresh => UpdateRoute::Journal,

        Message::AddComparisonChart
        | Message::AddPairRatioChart
        | Message::SpaghettiReload(_)
        | Message::SpaghettiSwitchTimeframe(_, _)
        | Message::SpaghettiCandlesLoaded(_, _)
        | Message::SpaghettiWsCandleUpdate(_, _)
        | Message::SpaghettiWsCandleLagged(_, _)
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
        | Message::WalletDetailsLoaded(_, _, _, _)
        | Message::WalletDetailsWsUpdate(_, _)
        | Message::WalletTrackerInputChanged(_)
        | Message::WalletTrackerLabelInputChanged(_)
        | Message::WalletTrackerAdd
        | Message::WalletTrackerMute(_)
        | Message::WalletTrackerUnmute(_)
        | Message::WalletTrackerRemove(_)
        | Message::WalletTrackerLabelChanged(_, _)
        | Message::WalletTrackerRefresh
        | Message::WalletTrackerRefreshDue
        | Message::WalletTrackerRefreshOne(_)
        | Message::WalletTrackerRefreshOrdersDue
        | Message::WalletTrackerRefreshOrders(_)
        | Message::WalletTrackerLoaded(_, _, _)
        | Message::WalletTrackerBatchLoaded(_, _)
        | Message::WalletTrackerOrdersLoaded(_, _, _) => UpdateRoute::WalletTracker,

        Message::RefreshPortfolio
        | Message::PortfolioLoaded(_, _, _)
        | Message::RefreshIncome
        | Message::IncomeLoaded(_, _, _)
        | Message::SetPortfolioPnlValueMode(_)
        | Message::SetPortfolioScope(_)
        | Message::SetPortfolioWindow(_) => UpdateRoute::PortfolioIncome,

        Message::SetDrawingTool(_, _, _)
        | Message::AddAnnotation(_, _)
        | Message::RemoveAnnotation(_, _)
        | Message::UpdateAnnotation(_, _)
        | Message::SelectAnnotation(_, _)
        | Message::RestyleAnnotation(_, _, _)
        | Message::ClearDrawingTool(_, _) => UpdateRoute::Annotations,

        Message::ChartFocused(_)
        | Message::ChartReload(_)
        | Message::ChartResetView(_, _)
        | Message::ChartSwitchTimeframe(_, _)
        | Message::ToggleMacroMenu(_)
        | Message::ToggleMacroIndicator(_, _)
        | Message::ToggleChartEarningsMarkers(_)
        | Message::ChartEarningsEventsLoaded(_, _, _)
        | Message::MacroCandlesLoaded(_, _, _, _, _)
        | Message::ChartCandlesLoaded(_, _)
        | Message::ChartSecondaryCandlesLoaded(_, _)
        | Message::ChartFundingHistoryLoaded(_, _)
        | Message::ChartWsCandleUpdate(_, _, _, _, _)
        | Message::ChartWsCandleLagged(_, _, _, _, _)
        | Message::ChartPriceFlashTick
        | Message::ChartHudOrderAnimationTick
        | Message::ChartHudArmToggled(_, _)
        | Message::ChartHudControlChanged(_, _, _, _)
        | Message::ChartHudSafetyTick
        | Message::ChartHoverStateChanged(_, _, _, _, _)
        | Message::ChartOrderCancelHoverAnimationTick
        | Message::ChartEarningsMarkerHoverAnimationTick
        | Message::ChartWsAssetCtxUpdate(_, _, _, _)
        | Message::ChartWsAssetCtxLagged(_, _, _, _)
        | Message::ChartAssetContextRestFetched(_, _, _)
        | Message::ChartViewportChanged(_, _, _)
        | Message::ChartFundingPanelHeightChanged(_, _, _)
        | Message::ChartSessionPanelHeightChanged(_, _, _)
        | Message::ToggleFundingRateDisplayMode(_)
        | Message::FundingRefreshTick
        | Message::ToggleOpenInterestNotional(_)
        | Message::ToggleAssetVolumeNotional(_)
        | Message::ToggleOutcomeVolumeNotional(_)
        | Message::ChartSymbolSelected(_, _)
        | Message::ChartSecondarySymbolSelected(_, _)
        | Message::ChartSecondarySymbolRemoved(_)
        | Message::ToggleChartInvert(_)
        | Message::ToggleChartTradeMarkers(_)
        | Message::ToggleChartHeaderCollapsed(_)
        | Message::ToggleChartDrawingToolbar(_)
        | Message::ChartOpenEditor(_)
        | Message::ChartCloseEditor(_)
        | Message::ChartEditorSearchChanged(_, _)
        | Message::ChartEditorSubmit(_)
        | Message::ChartSecondaryOpenEditor(_)
        | Message::ChartSecondaryCloseEditor(_)
        | Message::ChartSecondaryEditorSearchChanged(_, _)
        | Message::ChartSecondaryEditorSubmit(_)
        | Message::OpenDetachedChart(_)
        | Message::AddChart(_) => UpdateRoute::Chart,

        Message::PositionsSortChanged(_)
        | Message::ToggleHiddenPosition(_)
        | Message::ToggleShowHiddenPositions
        | Message::OpenPnlCard(_)
        | Message::SetPnlCardDisplayMode(_, _)
        | Message::SetPnlCardPercentMode(_, _)
        | Message::TogglePnlCardPricePrivacy(_, _)
        | Message::TogglePnlCardPositionSize(_, _)
        | Message::CopyPnlCard(_)
        | Message::PnlCardCopied(_)
        | Message::SavePnlCard(_)
        | Message::PnlCardSaved(_)
        | Message::WalletKeyInputChanged(_)
        | Message::WalletAddressInputChanged(_)
        | Message::ToggleAccountPicker
        | Message::AccountPickerSelected(_)
        | Message::AccountPickerRenameToggled(_)
        | Message::AccountPickerLabelChanged(_, _)
        | Message::AddAccount
        | Message::GhostWallet(_)
        | Message::ForgetGhostAccount(_)
        | Message::DeleteSavedAccount(_)
        | Message::SaveCredentials
        | Message::ConnectWallet
        | Message::DisconnectWallet
        | Message::AccountDataLoaded(_, _, _)
        | Message::RetryTwapReconciliationAccountData(_)
        | Message::RefreshAccountData
        | Message::AccountRefreshBackoffElapsed(_)
        | Message::AllMidsBootstrapLoaded(_, _)
        | Message::WsUserDataUpdate(_, _) => UpdateRoute::Account,

        Message::HydromancerKeyInputChanged(_)
        | Message::SaveHydromancerKey
        | Message::ReconnectLiquidations
        | Message::ReconnectTrackedTrades
        | Message::WsHydromancerLiquidation { .. }
        | Message::WsHydromancerTrackedTrades { .. }
        | Message::ClearLiquidations
        | Message::LiquidationFeedScrolled(_)
        | Message::ClearTrackedTrades
        | Message::RefreshTelegramFeed
        | Message::TelegramFeedRefreshTick
        | Message::TelegramFeedLoaded(_, _, _)
        | Message::TelegramAvatarLoaded(_, _, _, _)
        | Message::ToggleTelegramFastFeed
        | Message::TelegramFastApiIdChanged(_)
        | Message::TelegramFastApiHashChanged(_)
        | Message::TelegramFastPhoneChanged(_)
        | Message::TelegramFastCodeChanged(_)
        | Message::TelegramFastPasswordChanged(_)
        | Message::TelegramFastRequestCode
        | Message::TelegramFastSubmitCode
        | Message::TelegramFastSubmitPassword
        | Message::TelegramFastSignOut
        | Message::TelegramFastAuthResult(_, _)
        | Message::TelegramFastFeedEvent(_, _)
        | Message::TelegramFeedChannelInputChanged(_)
        | Message::TelegramFeedAddChannel
        | Message::TelegramPrivateChannelsRefresh
        | Message::TelegramPrivateChannelsLoaded(_, _)
        | Message::TelegramFeedAddPrivateChannel(_)
        | Message::ToggleTelegramPrivateChannelCandidatesExpanded
        | Message::TelegramFeedRemoveChannel(_)
        | Message::ToggleTelegramFeedChannelsExpanded
        | Message::ToggleTelegramFeedNotifications
        | Message::ToggleTelegramFeedOutcomeMarkets => UpdateRoute::Feed,

        Message::HyperdashKeyInputChanged(_)
        | Message::SaveHyperdashKey
        | Message::ToggleLiquidationOverlay(_)
        | Message::ChartLiquidationLoaded(_, _, _)
        | Message::RefreshLiquidations
        | Message::LiquidationsDistributionLoaded(_, _, _)
        | Message::RefreshLiquidationsDistribution
        | Message::LiquidationsDistributionSearchChanged(_)
        | Message::ToggleLiquidationsDistributionSymbolPicker
        | Message::LiquidationsDistributionSymbolSelected(_)
        | Message::LiquidationsDistributionZoomed { .. }
        | Message::ResetLiquidationsDistributionZoom
        | Message::ToggleHeatmapOverlay(_)
        | Message::ChartHeatmapLoaded(_, _, _)
        | Message::RefreshHeatmap => UpdateRoute::Hyperdash,
    }
}

#[cfg(test)]
mod tests;
