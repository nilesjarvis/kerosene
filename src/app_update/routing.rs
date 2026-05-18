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
        | Message::ToggleTrackedTradeSettingsMenu
        | Message::ToggleLiquidationFeedAggregation
        | Message::ToggleLiquidationChart
        | Message::ToggleLiquidationSummary
        | Message::ToggleLiquidationSettingsMenu
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
        | Message::ToggleDesktopNotifications => UpdateRoute::Chrome,

        Message::OrderPriceChanged(_)
        | Message::SetMidPrice
        | Message::OrderBookPriceSelected { .. }
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
        | Message::TwapOrderStatusLoaded { .. }
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
        | Message::QuickOrderPercentageChanged(_, _)
        | Message::QuickOrderToggleDenomination(_)
        | Message::QuickOrderToggleType(_)
        | Message::CloseQuickOrder(_)
        | Message::SubmitQuickOrder(_, _)
        | Message::QuickOrderResult(_)
        | Message::EscapePressed
        | Message::MoveOrderDragStarted { .. }
        | Message::MoveOrder { .. }
        | Message::MoveOrderModifyResult { .. }
        | Message::ChaseRestingOrder { .. } => UpdateRoute::Order,

        Message::ToggleFavourite(_)
        | Message::SymbolsLoaded(_)
        | Message::LiveWatchlistSortChanged(_, _)
        | Message::LiveWatchlistColumnToggled(_, _, _)
        | Message::AddOrderBookPane
        | Message::AddLiveWatchlistPane
        | Message::AddPositioningInfoPane
        | Message::PositioningInfoPageChanged(_, _)
        | Message::PositioningInfoSearchChanged(_, _)
        | Message::PositioningInfoSymbolSelected(_, _)
        | Message::PositioningInfoSideChanged(_, _)
        | Message::PositioningInfoSortChanged(_, _)
        | Message::ClearPositioningInfoFilters(_)
        | Message::RefreshPositioningInfoPane(_)
        | Message::RefreshPositioningInfo
        | Message::PositioningInfoWsAssetCtxUpdate(_, _)
        | Message::PositioningInfoLoaded(_, _)
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
        | Message::BookLoaded { .. }
        | Message::OrderBookWsAssetCtxUpdate(_, _)
        | Message::WsBookUpdate { .. }
        | Message::SetBookTickSize(_, _)
        | Message::ToggleOrderBookSettings(_)
        | Message::ToggleOrderBookCenterOnMid(_)
        | Message::ToggleOrderBookSpreadChart(_)
        | Message::OrderBookSpreadChartResize(_, _)
        | Message::OrderBookSearchChanged(_, _)
        | Message::OrderBookSetMode(_, _)
        | Message::SetOrderBookDisplayMode(_, _) => UpdateRoute::Market,

        Message::ThemeChanged(_)
        | Message::PaneBorderThicknessChanged(_)
        | Message::PaneCornerRadiusChanged(_)
        | Message::MutedTickerInputChanged(_)
        | Message::MuteTicker
        | Message::UnmuteTicker(_)
        | Message::MarketUniverseChanged(_)
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

        Message::ToggleChartScreenshotMenu(_)
        | Message::ToggleChartScreenshotObscurePositionEntry(_)
        | Message::ToggleChartScreenshotHidePositionsAndOrders(_)
        | Message::OpenChartScreenshot(_)
        | Message::ChartScreenshotBoundsResolved(_, _, _)
        | Message::ChartScreenshotCaptured(_, _, _)
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
        | Message::SetPortfolioPnlValueMode(_)
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
        | Message::ChartPriceFlashTick
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
    use crate::pnl_card::{PnlCardDisplayMode, PnlCardPercentMode, PnlCardTarget};
    use crate::portfolio_state::PnlValueDisplayMode;

    #[test]
    fn routes_messages_with_known_overlap_to_existing_update_modules() {
        let window_id = iced::window::Id::unique();

        assert_eq!(message_route(&Message::Tick), UpdateRoute::Calendar);
        assert_eq!(
            message_route(&Message::CalendarImpactFilterChanged(
                crate::calendar_state::CalendarImpactFilter::All,
            )),
            UpdateRoute::Chrome
        );
        assert_eq!(message_route(&Message::ToggleHidePnl), UpdateRoute::Chrome);
        assert_eq!(
            message_route(&Message::SetPortfolioPnlValueMode(
                PnlValueDisplayMode::Percent,
            )),
            UpdateRoute::PortfolioIncome
        );
        assert_eq!(
            message_route(&Message::ToggleLayoutMenu),
            UpdateRoute::Panes
        );
        assert_eq!(
            message_route(&Message::UpdateActiveLayout),
            UpdateRoute::Layout
        );
        assert_eq!(
            message_route(&Message::LayoutRenameToggled(0)),
            UpdateRoute::Layout
        );
        assert_eq!(
            message_route(&Message::LayoutRenameChanged("Main".to_string())),
            UpdateRoute::Layout
        );
        assert_eq!(
            message_route(&Message::LayoutRenameSubmitted(0)),
            UpdateRoute::Layout
        );
        assert_eq!(
            message_route(&Message::ToggleHiddenPosition("BTC".to_string())),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::OpenPnlCard(PnlCardTarget::Position(
                "BTC".to_string(),
            ))),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::SetPnlCardDisplayMode(
                window_id,
                PnlCardDisplayMode::Both,
            )),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::SetPnlCardPercentMode(
                window_id,
                PnlCardPercentMode::Leveraged,
            )),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::TogglePnlCardPricePrivacy(window_id, true)),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::TogglePnlCardPositionSize(window_id, true)),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::CopyPnlCard(window_id)),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::PnlCardCopied(Ok(()))),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::SavePnlCard(window_id)),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::PnlCardSaved(Ok(None))),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::AccountPickerRenameToggled(0)),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::AccountPickerLabelChanged(0, "Main".to_string())),
            UpdateRoute::Account
        );
        assert_eq!(
            message_route(&Message::DismissOrderStatus),
            UpdateRoute::Order
        );
        assert_eq!(
            message_route(&Message::OrderBookPriceSelected {
                id: 7,
                price: "100.5".to_string(),
            }),
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
        assert_eq!(
            message_route(&Message::ToggleChartScreenshotObscurePositionEntry(true)),
            UpdateRoute::ChartScreenshot
        );
        assert_eq!(
            message_route(&Message::ToggleChartScreenshotHidePositionsAndOrders(true)),
            UpdateRoute::ChartScreenshot
        );
    }
}
