use crate::account::{AccountData, AssetContext, WalletDetailsData, WalletTrackerSnapshot};
use crate::account_analytics::{IncomeSnapshot, PortfolioHistory};
use crate::account_state::{BottomTab, PositionsSortColumn};
use crate::alfred_state::{AlfredCommandId, AlfredSelectionStep};
use crate::annotations::{Annotation, AnnotationId, AnnotationStyle, DrawingTool};
use crate::api::{self, Candle, OrderBook};
use crate::calendar_state::{CalendarImpactFilter, CalendarWindowFilter};
use crate::chart::ChartViewport;
use crate::chart_screenshot::ChartScreenshotState;
use crate::chart_state::{CandleFetchRequest, ChartId, ChartSurfaceId, FundingFetchRequest};
use crate::config;
use crate::hydromancer_api::FundingRatePoint;
use crate::hype_etf_state::{HypeEtfData, HypeEtfView};
use crate::hype_unstaking_state::{
    HypeUnstakingAmountFilter, HypeUnstakingQueueData, HypeUnstakingSortField,
    HypeUnstakingWindowFilter,
};
use crate::hyperdash_api::{LiquidationHeatmap, LiquidationLevel};
use crate::journal;
use crate::liquidations_distribution_state::LiquidationDistributionZoomAnchor;
use crate::market_state::{
    LiveWatchlistId, OrderBookDisplayMode, OrderBookId, OrderBookSymbolMode,
    SymbolSearchMarketFilter, SymbolSearchSortMode,
};
use crate::order_execution::{
    AdvancedOrderStartSnapshot, HudOrderRequest, OneShotPlacementContext,
    OrderLeverageSubmissionSnapshot, PendingLeverageUpdateContext, QuickOrderRecovery,
    QuickOrderSubmissionSnapshot, TicketOrderSubmissionSnapshot, TwapOrderStartSnapshot,
};
use crate::pane_management::AddWidgetPlacement;
use crate::pnl_card::{PnlCardDisplayMode, PnlCardPercentMode, PnlCardTarget};
use crate::portfolio_state::{PnlValueDisplayMode, PortfolioScope, PortfolioWindow};
use crate::positioning_state::{
    PositioningInfoChangeTimeframe, PositioningInfoId, PositioningInfoPage, PositioningInfoSide,
    PositioningInfoSortField,
};
use crate::read_data_provider::{
    AccountDataRequestContext, MarketDataSourceContext, ReadDataRequestContext,
};
use crate::screener_state::{ScreenerExchangeFilter, ScreenerSortColumn};
use crate::session_data_state::{
    SessionDataCandles, SessionDataId, SessionDataLookback, SessionDataRequest,
};
use crate::settings_state::{SettingsTab, ThemeSettingsPage};
use crate::signing::{ExchangeResponse, OrderKind};
use crate::spaghetti;
use crate::spaghetti_state::SpaghettiWsCandleContext;
use crate::spaghetti_state::{SpaghettiCandleFetch, SpaghettiChartId};
use crate::telegram_feed::{
    TelegramFastAuthOutcome, TelegramFastFeedEvent, TelegramFeedPage,
    TelegramPrivateChannelCandidate,
};
use crate::timeframe::Timeframe;
use crate::ws::WsUserData;
use iced::widget::pane_grid;
use iced::{Point, Size, window};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use zeroize::Zeroizing;

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct SecretInput(Zeroizing<String>);

impl SecretInput {
    pub(crate) fn into_zeroizing(self) -> Zeroizing<String> {
        self.0
    }
}

impl From<String> for SecretInput {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&str> for SecretInput {
    fn from(value: &str) -> Self {
        Self(value.to_string().into())
    }
}

impl fmt::Debug for SecretInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretInput(<redacted>)")
    }
}

#[derive(Clone)]
pub(crate) struct TelegramFastAuthMessageResult(Box<Result<TelegramFastAuthOutcome, String>>);

impl TelegramFastAuthMessageResult {
    pub(crate) fn new(result: Result<TelegramFastAuthOutcome, String>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<TelegramFastAuthOutcome, String> {
        *self.0
    }
}

impl From<Result<TelegramFastAuthOutcome, String>> for TelegramFastAuthMessageResult {
    fn from(result: Result<TelegramFastAuthOutcome, String>) -> Self {
        Self::new(result)
    }
}

impl fmt::Debug for TelegramFastAuthMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(outcome) => f.debug_tuple("Ok").field(outcome).finish(),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SaveLayout(String),
    LoadLayout(config::SavedLayout),
    DeleteLayout(String),
    UpdateActiveLayout,
    LayoutRenameToggled(usize),
    LayoutRenameChanged(String),
    LayoutRenameSubmitted(usize),
    ExportLayout(config::SavedLayout),
    ImportLayout,
    LayoutImported(Result<config::SavedLayout, String>),
    LayoutExported(Result<(), String>),
    ExportWalletLabels,
    ImportWalletLabels,
    WalletLabelsExported(Result<(), String>),
    WalletLabelsImported(Result<config::WalletLabelsExport, String>),
    LayoutInputChanged(String),
    LiveWatchlistSearchChanged(LiveWatchlistId, String),
    LiveWatchlistContextsLoaded(
        u64,
        Vec<String>,
        u64,
        Result<HashMap<String, crate::api::WatchlistContext>, String>,
    ),
    LiveWatchlistHistoryLoaded(
        u64,
        Vec<String>,
        u64,
        Result<HashMap<String, (f64, f64, f64)>, String>,
    ),
    LiveWatchlistAddSymbol(LiveWatchlistId, String),
    LiveWatchlistRemoveSymbol(LiveWatchlistId, String),
    LiveWatchlistRefreshTick,
    LiveWatchlistSortChanged(LiveWatchlistId, config::LiveWatchlistSortColumn),
    LiveWatchlistColumnToggled(LiveWatchlistId, config::LiveWatchlistColumn, bool),
    ToggleLiveWatchlistSettings(LiveWatchlistId),
    AddLiveWatchlistPane,
    AddPositioningInfoPane,
    PositioningInfoPageChanged(PositioningInfoId, PositioningInfoPage),
    PositioningInfoSearchChanged(PositioningInfoId, String),
    TogglePositioningInfoSymbolPicker(PositioningInfoId),
    PositioningInfoSymbolSelected(PositioningInfoId, String),
    PositioningInfoSideChanged(PositioningInfoId, PositioningInfoSide),
    PositioningInfoSortChanged(PositioningInfoId, PositioningInfoSortField),
    PositioningInfoChangeTimeframeChanged(PositioningInfoId, PositioningInfoChangeTimeframe),
    ClearPositioningInfoFilters(PositioningInfoId),
    RefreshPositioningInfoPane(PositioningInfoId),
    RefreshPositioningInfo,
    PositioningInfoWsAssetCtxUpdate(String, MarketDataSourceContext, AssetContext),
    PositioningInfoWsAssetCtxLagged(String, MarketDataSourceContext, u64),
    PositioningInfoLoaded(
        String,
        u64,
        Box<Result<crate::hyperdash_api::TickerPositions, String>>,
    ),
    PositioningInfoChangeLoaded(
        String,
        u64,
        Box<Result<crate::hyperdash_api::PerpDeltas, String>>,
    ),
    AddOrderBookPane,
    AddAdvancedOrdersPane,
    PositionsSortChanged(PositionsSortColumn),

    ToggleAccountPicker,
    AccountPickerSelected(usize),
    AccountPickerRenameToggled(usize),
    AccountPickerLabelChanged(usize, String),
    AddAccount,
    GhostWallet(String),
    ForgetGhostAccount(usize),
    DeleteSavedAccount(usize),
    SaveCredentials,
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    PaneClicked(pane_grid::Pane),
    SwitchBottomTab(BottomTab),
    OrderPriceChanged(String),
    SetMidPrice,
    OrderBookPriceSelected {
        id: OrderBookId,
        price: String,
    },
    OrderQuantityChanged(String),
    SetOrderKind(OrderKind),
    ToggleOrderDenomination,
    OrderPercentageChanged(f32),
    PrefillOutcomeSell(String),
    ToggleReduceOnly,
    ToggleOrderLeverageDropdown,
    OrderLeverageInputChanged(String),
    SetOrderLeverageCross(bool),
    SubmitOrderLeverage(OrderLeverageSubmissionSnapshot),
    OrderLeverageResult {
        context: PendingLeverageUpdateContext,
        result: Box<Result<ExchangeResponse, String>>,
    },
    TogglePresetsMenu,
    TogglePresetCurrency,
    TogglePresetEditMode,
    SetAddWidgetPlacement(AddWidgetPlacement),
    EditPresetStart(crate::signing::OrderKind, usize, String),
    EditPresetChanged(String),
    EditPresetSave(crate::signing::OrderKind, usize),
    ExecutePreset(crate::signing::OrderKind, crate::config::OrderPreset, bool),
    ToggleFavourite(String),
    ToggleTickerTape,
    TickerTapeTick,
    TickerTapeRefreshTick,
    TickerTapeContextsLoaded(
        u64,
        Vec<String>,
        u64,
        Result<HashMap<String, crate::api::WatchlistContext>, String>,
    ),
    // Add widget menu
    ToggleAddWidgetMenu,
    ToggleLayoutMenu,
    ToggleMacroMenu(ChartId),
    ToggleMacroIndicator(ChartId, String),
    ToggleChartEarningsMarkers(ChartId),
    ChartEarningsEventsLoaded(String, u64, Box<Result<Vec<api::SecEarningsEvent>, String>>),
    CloseAllMenus,
    AddPortfolioPane,
    AddIncomePane,
    AddComparisonChart,
    AddPairRatioChart,
    OpenSettingsWindow,
    OpenScreenerWindow,
    RefreshScreener,
    ForceRefreshScreener,
    RefreshScreenerHistory,
    ScreenerExchangeFilterChanged(ScreenerExchangeFilter),
    ScreenerSortChanged(ScreenerSortColumn),
    ScreenerContextsLoaded(
        u64,
        Vec<String>,
        u64,
        Result<HashMap<String, crate::api::WatchlistContext>, String>,
    ),
    ScreenerHistoryLoaded(
        u64,
        Vec<String>,
        u64,
        Result<HashMap<String, (f64, f64)>, String>,
    ),
    SettingsTabSelected(SettingsTab),
    ThemeSettingsPageSelected(ThemeSettingsPage),
    OpenUnlockCredentialsPopup,
    DismissUnlockCredentialsPopup,
    OpenCredentialStorageSettings,
    SecretStorageSelectionChanged(config::CredentialStorageMode),
    EncryptedSecretPasswordChanged(SecretInput),
    EncryptedSecretConfirmChanged(SecretInput),
    UnlockEncryptedSecrets,
    ApplySecretStorageSelection,
    ClearConfigs,
    ConfigsCleared(Result<config::ClearConfigSummary, String>),
    AddCalendarPane,
    AddLiquidationsPane,
    AddLiquidationsDistributionPane,
    AddTrackedTradesPane,
    AddTelegramFeedPane,
    AddOutcomesPane,
    AddHypeEtfsPane,
    AddHypeUnstakingQueuePane,
    AddSessionDataPane,
    SessionDataSearchChanged(SessionDataId, String),
    ToggleSessionDataSymbolPicker(SessionDataId),
    SessionDataSymbolSelected(SessionDataId, String),
    SessionDataLookbackChanged(SessionDataId, SessionDataLookback),
    RefreshSessionData(SessionDataId),
    SessionDataCandlesLoaded(SessionDataRequest, Result<SessionDataCandles, String>),
    AddTradingJournal,
    RefreshCalendar,
    CalendarLoaded(u64, Result<Vec<api::CalendarEvent>, String>),
    RefreshHypeEtfs,
    HypeEtfsRefreshTick,
    HypeEtfsViewChanged(HypeEtfView),
    HypeEtfsLoaded(u64, Box<Result<HypeEtfData, String>>),
    RefreshHypeUnstakingQueue,
    HypeUnstakingQueueRefreshTick,
    HypeUnstakingWindowChanged(HypeUnstakingWindowFilter),
    HypeUnstakingAmountFilterChanged(HypeUnstakingAmountFilter),
    HypeUnstakingSortChanged(HypeUnstakingSortField),
    ToggleHypeUnstakingMineOnly,
    ClearHypeUnstakingFilters,
    HypeUnstakingQueueLoaded(u64, Box<Result<HypeUnstakingQueueData, String>>),
    CalendarImpactFilterChanged(CalendarImpactFilter),
    CalendarWindowFilterChanged(CalendarWindowFilter),
    Tick,
    ThemeChanged(String),
    UiScaleChanged(f32),
    ToggleChartDottedBackground(bool),
    ChartDottedBackgroundOpacityChanged(f32),
    ChartHollowCandleModeChanged(config::ChartHollowCandleMode),
    ChartSeriesStyleChanged(config::ChartSeriesStyle),
    ToggleChartFisheye(bool),
    ChartFisheyeStrengthChanged(f32),
    ToggleChartChromaticAberration(bool),
    ChartChromaticAberrationStrengthChanged(f32),
    ToggleChartEdgeBlur(bool),
    ChartEdgeBlurStrengthChanged(f32),
    ChartCrosshairStyleChanged(config::ChartCrosshairStyle),
    ToggleChartCrosshairGuides(bool),
    ChartCrosshairScaleChanged(f32),
    ChartHudReadoutToggled(config::ChartHudReadoutElement, bool),
    ChartHudOrderSoundChanged(config::ChartHudOrderSound),
    ChartHudOrderSoundVolumeChanged(f32),
    ImportChartHudOrderSound,
    ChartHudOrderSoundImported(Result<Option<String>, String>),
    TestChartHudOrderSound,
    ToggleChartHudUiSounds(bool),
    ReadDataProviderChanged(config::ReadDataProvider),
    AlfredPopupScaleChanged(f32),
    DisplayFontChanged(config::DisplayFontConfig),
    MonospaceFontChanged(config::DisplayFontConfig),
    ImportDisplayFont,
    DisplayFontImported(Result<config::CustomFontConfig, String>),
    ImportMonospaceFont,
    MonospaceFontImported(Result<config::CustomFontConfig, String>),
    PaneBorderThicknessChanged(f32),
    PaneCornerRadiusChanged(f32),
    ToggleOuterWidgetBorder(bool),
    DefaultWidgetPaddingChanged(f32),
    FocusedWidgetPaddingChanged(f32),
    ResetFocusedWidgetPadding,
    ToggleCustomWindowChrome(bool),
    MutedTickerInputChanged(String),
    MuteTicker,
    UnmuteTicker(String),
    MarketUniverseChanged(config::MarketUniverseConfig),
    DisplayDenominationChanged(config::DisplayDenominationConfig),
    MarketSlippageInputChanged(String),
    SaveMarketSlippage,
    ExecuteHotkey(config::HotkeyAction),
    StartRecordingHotkey(config::HotkeyAction),
    ClearHotkey(config::HotkeyAction),
    ToggleAlfred,
    CloseAlfred,
    AlfredQueryChanged(String),
    AlfredSelectionMoved(AlfredSelectionStep),
    AlfredSubmit,
    AlfredCommandSelected(AlfredCommandId),
    OpenWalletTrackerWindow,
    OpenWalletDetailsWindow(String),
    RefreshWalletDetails(window::Id),
    WalletDetailsLoaded(
        window::Id,
        String,
        ReadDataRequestContext,
        Box<Result<WalletDetailsData, String>>,
    ),
    WalletDetailsWsUpdate(Option<String>, Box<WsUserData>),
    WindowOpened(window::Id),
    WindowClosed(window::Id),
    WindowResized(window::Id, Size),
    WindowMoved(window::Id, Point),
    WindowDrag(window::Id),
    WindowDragResize(window::Id, window::Direction),
    WindowMinimize(window::Id),
    WindowToggleMaximize(window::Id),
    WindowClose(window::Id),
    // Trading Journal
    JournalFillsLoaded {
        request_id: u64,
        account_key: Option<String>,
        address: String,
        result: Result<api::UserFillsPage, String>,
    },
    JournalRefresh,
    JournalClearCache,
    JournalEditStart(String, Option<String>),
    JournalEditCancel(String),
    JournalEditSave(String),
    JournalBufferChanged(String, bool, String),
    JournalFilterChanged(journal::JournalFilter),
    JournalSortChanged(journal::JournalSort),
    JournalPortfolioWindowChanged(PortfolioWindow),
    JournalChartRevealTick,
    JournalToggleAllAssets,
    JournalToggleAccountValueChart(bool),
    JournalToggleIncludeFeesInPnl,
    JournalSnapshotToggle(String),
    JournalSnapshotLoaded {
        account_key: Option<String>,
        address: String,
        request: journal::JournalTradeSnapshotRequest,
        result: Result<Vec<Candle>, String>,
    },
    // Spaghetti chart
    SpaghettiSwitchTimeframe(SpaghettiChartId, Timeframe),
    SpaghettiReload(SpaghettiChartId),
    SpaghettiCandlesLoaded(SpaghettiCandleFetch, Result<Vec<Candle>, String>),
    SpaghettiWsCandleUpdate(SpaghettiWsCandleContext, Candle),
    SpaghettiWsCandleLagged(SpaghettiWsCandleContext, u64),
    SpaghettiOpenEditor(SpaghettiChartId),
    SpaghettiCloseEditor(SpaghettiChartId),
    SpaghettiEditorSearchChanged(SpaghettiChartId, String),
    SpaghettiAddSymbol(SpaghettiChartId, String),
    SpaghettiRemoveSymbol(SpaghettiChartId, String),
    SpaghettiSetSession(SpaghettiChartId, Option<spaghetti::Session>),
    SpaghettiSetSessionGranularityAuto(SpaghettiChartId),
    SpaghettiResetView(SpaghettiChartId),
    ToggleSpaghettiStyleMenu(SpaghettiChartId),
    ToggleSpaghettiLabels(SpaghettiChartId),
    SpaghettiSetColorMode(SpaghettiChartId, spaghetti::ComparisonColorMode),
    PairSetCandleMode(SpaghettiChartId, bool),
    WalletTrackerInputChanged(String),
    WalletTrackerLabelInputChanged(String),
    WalletTrackerAdd,
    WalletTrackerMute(String),
    WalletTrackerUnmute(String),
    WalletTrackerRemove(String),
    WalletTrackerLabelChanged(String, String),
    WalletTrackerRefresh,
    WalletTrackerRefreshDue,
    WalletTrackerRefreshOne(String),
    WalletTrackerRefreshOrdersDue,
    WalletTrackerRefreshOrders(String),
    WalletTrackerLoaded(
        String,
        ReadDataRequestContext,
        Box<Result<WalletTrackerSnapshot, String>>,
    ),
    WalletTrackerBatchLoaded(
        ReadDataRequestContext,
        Vec<(String, Result<WalletTrackerSnapshot, String>)>,
    ),
    WalletTrackerOrdersLoaded(String, ReadDataRequestContext, Box<Result<usize, String>>),
    RefreshPortfolio,
    PortfolioLoaded(String, u64, Box<Result<PortfolioHistory, String>>),
    RefreshIncome,
    IncomeLoaded(String, u64, Box<Result<IncomeSnapshot, String>>),
    ToggleIncomeAlerts,
    ToggleLiquidationAlerts,
    ToggleTrackedTradeAlerts,
    ToggleTrackedTradeAggregation,
    ToggleTrackedTradeSettingsMenu,
    ToggleLiquidationFeedAggregation,
    ToggleLiquidationChart,
    ToggleLiquidationSummary,
    ToggleLiquidationFollow,
    ToggleLiquidationSettingsMenu,
    LiquidationAlertThresholdChanged(String),
    SaveLiquidationAlertThreshold,
    SetPortfolioPnlValueMode(PnlValueDisplayMode),
    SetPortfolioScope(PortfolioScope),
    SetPortfolioWindow(PortfolioWindow),
    RefreshTelegramFeed,
    TelegramFeedRefreshTick,
    TelegramFeedLoaded(String, u64, Box<Result<TelegramFeedPage, String>>),
    TelegramAvatarLoaded(String, String, u64, Box<Result<Vec<u8>, String>>),
    ToggleTelegramFastFeed,
    TelegramFastApiIdChanged(String),
    TelegramFastApiHashChanged(SecretInput),
    TelegramFastPhoneChanged(String),
    TelegramFastCodeChanged(SecretInput),
    TelegramFastPasswordChanged(SecretInput),
    TelegramFastRequestCode,
    TelegramFastSubmitCode,
    TelegramFastSubmitPassword,
    TelegramFastSignOut,
    TelegramFastAuthResult(u64, TelegramFastAuthMessageResult),
    TelegramFastFeedEvent(u64, TelegramFastFeedEvent),
    TelegramFeedChannelInputChanged(String),
    TelegramFeedAddChannel,
    TelegramPrivateChannelsRefresh,
    TelegramPrivateChannelsLoaded(
        u64,
        Box<Result<Vec<TelegramPrivateChannelCandidate>, String>>,
    ),
    TelegramFeedAddPrivateChannel(i64),
    ToggleTelegramPrivateChannelCandidatesExpanded,
    TelegramFeedRemoveChannel(String),
    ToggleTelegramFeedChannelsExpanded,
    ToggleTelegramFeedNotifications,
    ToggleTelegramFeedOutcomeMarkets,
    // Drawing tools
    SetDrawingTool(ChartId, ChartSurfaceId, Option<DrawingTool>),
    AddAnnotation(ChartId, Annotation),
    RemoveAnnotation(ChartId, AnnotationId),
    UpdateAnnotation(ChartId, Annotation),
    SelectAnnotation(ChartId, Option<AnnotationId>),
    RestyleAnnotation(ChartId, AnnotationId, AnnotationStyle),
    ClearDrawingTool(ChartId, ChartSurfaceId),
    // Notifications
    DismissToast(u64),
    ToastPositionChanged(config::ToastPosition),
    ToggleToastAnimations(bool),
    ToastAnimationTick,
    CopyToClipboard(String),
    WalletAddressActionsHovered(String),
    WalletAddressActionsExited(String),
    TickToastCleanup,
    NoOp,
    SpinnerTick,
    StatusBarTick,
    ConfigSaved(Result<(), String>),
    ToggleSound,
    ToggleDesktopNotifications,
    ToggleOptimisticAccountUpdates(bool),
    PlaceOrder {
        is_buy: bool,
        snapshot: TicketOrderSubmissionSnapshot,
    },
    OrderResult {
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Box<Result<ExchangeResponse, String>>,
    },
    DismissOrderStatus,
    CancelOrder {
        coin: String,
        oid: u64,
    },
    CancelResult {
        account_address: String,
        pending_indicator_id: Option<u64>,
        result: Box<Result<ExchangeResponse, String>>,
    },
    CancelOrderStatusLoaded {
        account_address: String,
        oid: u64,
        symbol: String,
        result: Box<Result<api::OrderStatusResult, String>>,
    },
    ToggleCloseMenu(String),
    ToggleHiddenPosition(String),
    ToggleShowHiddenPositions,
    OpenPnlCard(PnlCardTarget),
    SetPnlCardDisplayMode(window::Id, PnlCardDisplayMode),
    SetPnlCardPercentMode(window::Id, PnlCardPercentMode),
    TogglePnlCardPricePrivacy(window::Id, bool),
    TogglePnlCardPositionSize(window::Id, bool),
    CopyPnlCard(window::Id),
    PnlCardCopied(Result<(), String>),
    SavePnlCard(window::Id),
    PnlCardSaved(Result<Option<PathBuf>, String>),
    ClosePosition {
        coin: String,
        fraction: f64,
        use_market: bool,
    },
    ClosePositionResult {
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Box<Result<ExchangeResponse, String>>,
    },
    NukePositions,
    NukeResult {
        execution_id: u64,
        context: OneShotPlacementContext,
        result: Box<Result<ExchangeResponse, String>>,
    },
    NukePlacementStatusLoaded {
        execution_id: u64,
        context: OneShotPlacementContext,
        result: Box<Result<api::OrderStatusResult, String>>,
    },
    OneShotPlacementStatusLoaded {
        request_id: u64,
        context: OneShotPlacementContext,
        result: Box<Result<api::OrderStatusResult, String>>,
    },
    StartChase {
        is_buy: bool,
        snapshot: AdvancedOrderStartSnapshot,
    },
    StopChase,
    StopChaseById(u64),
    StopAllAdvancedOrders,
    TwapDurationChanged(String),
    TwapSlicesChanged(String),
    TwapMinPriceChanged(String),
    TwapMaxPriceChanged(String),
    TwapRandomizeToggled(bool),
    StartTwap {
        is_buy: bool,
        snapshot: TwapOrderStartSnapshot,
    },
    StopTwap(u64),
    TwapTick,
    TwapBookUpdate {
        twap_id: u64,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        book: OrderBook,
    },
    TwapBookLagged {
        twap_id: u64,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        skipped: u64,
    },
    TwapSliceResult {
        twap_id: u64,
        result: Box<Result<ExchangeResponse, String>>,
    },
    TwapUnexpectedCancelResult {
        twap_id: u64,
        oid: Option<u64>,
        cloid: Option<String>,
        result: Box<Result<ExchangeResponse, String>>,
    },
    TwapUnexpectedCancelRetryDue {
        twap_id: u64,
        oid: Option<u64>,
        cloid: Option<String>,
        attempt: u32,
    },
    TwapOrderStatusLoaded {
        twap_id: u64,
        cloid: String,
        result: Box<Result<api::OrderStatusResult, String>>,
    },
    OpenTwapDetails(u64),
    OpenAdvancedOrderHistory(String),
    ChaseInitialBookLoaded {
        chase_id: u64,
        result: Box<Result<OrderBook, String>>,
    },
    ChaseBookUpdate {
        chase_id: u64,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        book: OrderBook,
    },
    ChaseBookLagged {
        chase_id: u64,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        skipped: u64,
    },
    ChaseRepriceTick,
    ChasePlaceResult {
        chase_id: u64,
        result: Box<Result<ExchangeResponse, String>>,
    },
    ChaseModifyResult {
        chase_id: u64,
        oid: u64,
        result: Box<Result<ExchangeResponse, String>>,
    },
    ChaseCancelResult {
        chase_id: u64,
        oid: u64,
        result: Box<Result<ExchangeResponse, String>>,
    },
    ChaseOrderStatusLoaded {
        chase_id: u64,
        cloid: String,
        result: Box<Result<api::OrderStatusResult, String>>,
    },
    ChaseOrderOidStatusLoaded {
        chase_id: u64,
        oid: u64,
        result: Box<Result<api::OrderStatusResult, String>>,
    },
    ChaseRestingOrder {
        coin: String,
        oid: u64,
    },
    // Per-chart messages (keyed by ChartId)
    ChartFocused(ChartId),
    ChartSwitchTimeframe(ChartId, Timeframe),
    ChartReload(ChartId),
    ChartResetView(ChartId, ChartSurfaceId),
    ChartCandlesLoaded(CandleFetchRequest, Result<Vec<Candle>, String>),
    ChartFundingHistoryLoaded(
        FundingFetchRequest,
        Box<Result<Vec<FundingRatePoint>, String>>,
    ),
    MacroCandlesLoaded(ChartId, u64, String, Timeframe, Result<Vec<Candle>, String>),
    ChartWsCandleUpdate(ChartId, String, String, MarketDataSourceContext, Candle),
    ChartWsCandleLagged(ChartId, String, String, MarketDataSourceContext, u64),
    ChartPriceFlashTick,
    ChartHudOrderAnimationTick,
    ChartHudArmToggled(ChartId, ChartSurfaceId),
    /// HUD selector control pressed: control, plus whether the value changed
    /// (sounds play on change; the weapon-selector popup opens either way).
    ChartHudControlChanged(ChartId, ChartSurfaceId, crate::sound::HudUiSound, bool),
    ChartHudSafetyTick,
    ChartHoverStateChanged(ChartId, ChartSurfaceId, Option<u64>, bool, Option<u64>),
    ChartOrderCancelHoverAnimationTick,
    ChartEarningsMarkerHoverAnimationTick,
    ChartWsAssetCtxUpdate(ChartId, String, MarketDataSourceContext, AssetContext),
    ChartWsAssetCtxLagged(ChartId, String, MarketDataSourceContext, u64),
    /// Result of the REST `metaAndAssetCtxs` fallback fetch for a chart symbol
    /// (chart id, symbol the fetch was issued for, fetched context).
    ChartAssetContextRestFetched(ChartId, String, Result<Option<AssetContext>, String>),
    ChartViewportChanged(ChartId, ChartSurfaceId, ChartViewport),
    ChartFundingPanelHeightChanged(ChartId, u16, bool),
    ChartSessionPanelHeightChanged(ChartId, u16, bool),
    ToggleFundingRateDisplayMode(ChartId),
    FundingRefreshTick,
    ToggleOpenInterestNotional(ChartId),
    ToggleAssetVolumeNotional(ChartId),
    ToggleOutcomeVolumeNotional(ChartId),
    ChartSymbolSelected(ChartId, String),
    ToggleChartInvert(ChartId),
    ToggleChartTradeMarkers(ChartId),
    ToggleChartHeaderCollapsed(ChartId),
    ToggleChartDrawingToolbar(ChartId),
    OpenDetachedChart(ChartId),
    ChartOpenEditor(ChartId),
    ChartCloseEditor(ChartId),
    ChartEditorSearchChanged(ChartId, String),
    ChartEditorSubmit(ChartId),
    ToggleChartScreenshotMenu(ChartId, ChartSurfaceId),
    ToggleChartScreenshotObscurePositionEntry(bool),
    ToggleChartScreenshotHidePositionsAndOrders(bool),
    OpenChartScreenshot(ChartId, ChartSurfaceId),
    ChartScreenshotBoundsResolved(u64, ChartId, ChartSurfaceId, Option<iced::Rectangle>),
    ChartScreenshotCaptured(u64, ChartId, Result<ChartScreenshotState, String>),
    CopyChartScreenshot,
    ChartScreenshotCopied(Result<(), String>),
    SaveChartScreenshot,
    ChartScreenshotSaved(Result<Option<PathBuf>, String>),
    CloseChartScreenshotWindow,
    // Hotkeys related messages
    KeyboardEvent(window::Id, iced::keyboard::Event, iced::event::Status),
    AddChart(pane_grid::Pane),
    ClosePane(pane_grid::Pane),
    ToggleHidePnl,
    // Quick order form (right-click on chart)
    OpenQuickOrder(ChartId, ChartSurfaceId, f64, f32, f32, f32, f32),
    QuickOrderQtyChanged(ChartId, String),
    QuickOrderPercentageChanged(ChartId, f32),
    QuickOrderToggleDenomination(ChartId),
    QuickOrderToggleType(ChartId),
    CloseQuickOrder(ChartId),
    SubmitQuickOrder {
        chart_id: ChartId,
        is_buy: bool,
        snapshot: QuickOrderSubmissionSnapshot,
    },
    QuickOrderResult {
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        recovery: Option<QuickOrderRecovery>,
        result: Box<Result<ExchangeResponse, String>>,
    },
    SubmitHudOrder(HudOrderRequest),
    HudOrderResult {
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Box<Result<ExchangeResponse, String>>,
    },
    EscapePressed(window::Id),
    // Order drag-to-move (from chart canvas)
    MoveOrderDragStarted {
        coin: String,
        oid: u64,
    },
    MoveOrder {
        coin: String,
        oid: u64,
        new_price: f64,
    },
    MoveOrderModifyResult {
        account_address: String,
        coin: String,
        oid: u64,
        pending_indicator_id: Option<u64>,
        result: Box<Result<ExchangeResponse, String>>,
    },
    MoveOrderStatusLoaded {
        account_address: String,
        coin: String,
        oid: u64,
        result: Box<Result<api::OrderStatusResult, String>>,
    },
    // Global messages
    SymbolsLoaded(Result<api::ExchangeSymbolsPayload, String>),
    ExchangeSymbolsRefreshTick,
    SymbolSearchChanged(String),
    SymbolSearchSortChanged(SymbolSearchSortMode),
    SymbolSearchMarketFilterChanged(SymbolSearchMarketFilter),
    SymbolSearchHip3DexFilterChanged(String),
    SymbolSearchContextsLoaded(
        u64,
        Vec<String>,
        u64,
        Result<HashMap<String, crate::api::WatchlistContext>, String>,
    ),
    OutcomeSearchChanged(String),
    OutcomeMarketGroupToggled(String),
    OutcomeVolumesLoaded(
        u64,
        Vec<String>,
        Result<HashMap<String, crate::api::OutcomeVolume24h>, String>,
    ),
    SymbolSelected(String),
    BookLoaded {
        request_id: u64,
        id: OrderBookId,
        coin: String,
        tick_size: f64,
        sigfigs: (Option<u8>, Option<u8>),
        result: Result<OrderBook, String>,
    },
    WsBookUpdate {
        id: OrderBookId,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        book: OrderBook,
    },
    OrderBookWsBookLagged {
        id: OrderBookId,
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        skipped: u64,
    },
    OrderBookWsAssetCtxUpdate {
        id: OrderBookId,
        coin: String,
        source_context: MarketDataSourceContext,
        ctx: AssetContext,
    },
    OrderBookWsAssetCtxLagged {
        id: OrderBookId,
        coin: String,
        source_context: MarketDataSourceContext,
        skipped: u64,
    },
    SetBookTickSize(OrderBookId, f64),
    ToggleOrderBookCenterOnMid(OrderBookId),
    ToggleOrderBookReverseSide(OrderBookId),
    ToggleOrderBookSettings(OrderBookId),
    ToggleOrderBookSpreadChart(OrderBookId),
    OrderBookSpreadChartResize(OrderBookId, f32),
    OrderBookSearchChanged(OrderBookId, String),
    OrderBookSetMode(OrderBookId, OrderBookSymbolMode),
    SetOrderBookDisplayMode(OrderBookId, OrderBookDisplayMode),
    WalletKeyInputChanged(SecretInput),
    WalletAddressInputChanged(String),
    HydromancerKeyInputChanged(SecretInput),
    SaveHydromancerKey,
    ReconnectLiquidations,
    ReconnectTrackedTrades,
    WsHydromancerLiquidation {
        hydromancer_key_generation: u64,
        reconnect_nonce: u64,
        message: crate::ws::HydromancerWsMessage,
    },
    WsHydromancerTrackedTrades {
        hydromancer_key_generation: u64,
        reconnect_nonce: u64,
        tracked_addresses: std::sync::Arc<[String]>,
        message: crate::ws::HydromancerWsMessage,
    },
    ClearLiquidations,
    LiquidationFeedScrolled(iced::widget::scrollable::Viewport),
    ClearTrackedTrades,
    ConnectWallet,
    DisconnectWallet,
    AccountDataLoaded(
        String,
        AccountDataRequestContext,
        Box<Result<AccountData, String>>,
    ),
    RetryTwapReconciliationAccountData(String),
    RefreshAccountData,
    AccountRefreshBackoffElapsed(u64),
    AllMidsBootstrapLoaded(String, Result<HashMap<String, f64>, String>),
    WsUserDataUpdate(Option<String>, Box<WsUserData>),
    // HyperDash liquidation heatmap
    HyperdashKeyInputChanged(SecretInput),
    SaveHyperdashKey,
    ToggleLiquidationOverlay(ChartId),
    ChartLiquidationLoaded(String, u64, Box<Result<LiquidationLevel, String>>),
    RefreshLiquidations,
    LiquidationsDistributionLoaded(String, u64, Box<Result<LiquidationLevel, String>>),
    RefreshLiquidationsDistribution,
    LiquidationsDistributionSearchChanged(String),
    ToggleLiquidationsDistributionSymbolPicker,
    LiquidationsDistributionSymbolSelected(String),
    LiquidationsDistributionZoomed {
        factor: f64,
        anchor: Option<LiquidationDistributionZoomAnchor>,
    },
    ResetLiquidationsDistributionZoom,
    // HyperDash historical liquidation heatmap
    ToggleHeatmapOverlay(ChartId),
    ChartHeatmapLoaded(String, u64, Box<Result<LiquidationHeatmap, String>>),
    RefreshHeatmap,
}

#[cfg(test)]
mod tests {
    use super::{Message, SecretInput, TelegramFastAuthMessageResult};

    #[test]
    fn secret_input_debug_redacts_value() {
        let rendered = format!("{:?}", SecretInput::from("super-secret"));

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("super-secret"));
    }

    #[test]
    fn secret_bearing_message_debug_redacts_value() {
        let messages = [
            Message::EncryptedSecretPasswordChanged("sentinel-secret".into()),
            Message::EncryptedSecretConfirmChanged("sentinel-secret".into()),
            Message::TelegramFastApiHashChanged("sentinel-secret".into()),
            Message::TelegramFastCodeChanged("sentinel-secret".into()),
            Message::TelegramFastPasswordChanged("sentinel-secret".into()),
            Message::TelegramFastAuthResult(
                1,
                TelegramFastAuthMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::WalletKeyInputChanged("sentinel-secret".into()),
            Message::HydromancerKeyInputChanged("sentinel-secret".into()),
            Message::HyperdashKeyInputChanged("sentinel-secret".into()),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"));
            assert!(!rendered.contains("sentinel-secret"));
        }
    }
}
