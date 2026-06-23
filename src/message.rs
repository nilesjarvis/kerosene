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
    TelegramPrivateChannelCandidate, telegram_private_channel_peer_id_from_key,
};
use crate::timeframe::Timeframe;
use crate::ws::WsUserData;
use iced::widget::pane_grid;
use iced::{Point, Size, window};
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
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

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedPhoneInput(Zeroizing<String>);

impl RedactedPhoneInput {
    pub(crate) fn into_string(self) -> String {
        self.0.to_string()
    }
}

impl From<String> for RedactedPhoneInput {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&str> for RedactedPhoneInput {
    fn from(value: &str) -> Self {
        Self(value.to_string().into())
    }
}

impl fmt::Debug for RedactedPhoneInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Phone(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedTelegramChannelKey(String);

impl RedactedTelegramChannelKey {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedTelegramChannelKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedTelegramChannelKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedTelegramChannelKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if telegram_private_channel_peer_id_from_key(&self.0).is_some() {
            f.write_str("TelegramChannel(<private>)")
        } else {
            f.debug_tuple("TelegramChannel").field(&self.0).finish()
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct RedactedAddress(String);

impl RedactedAddress {
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedAddress {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedAddress {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl AsRef<str> for RedactedAddress {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for RedactedAddress {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Debug for RedactedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Address(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedClipboardText(String);

impl RedactedClipboardText {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedClipboardText {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedClipboardText {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedClipboardText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClipboardText(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedOrderInput(String);

impl RedactedOrderInput {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedOrderInput {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedOrderInput {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedOrderInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OrderInput(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedAccountKey(Option<String>);

impl RedactedAccountKey {
    pub(crate) fn into_option(self) -> Option<String> {
        self.0
    }
}

impl From<Option<String>> for RedactedAccountKey {
    fn from(value: Option<String>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for RedactedAccountKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(_) => f.write_str("Some(<redacted>)"),
            None => f.write_str("None"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct RedactedAddressList(std::sync::Arc<[String]>);

impl RedactedAddressList {
    pub(crate) fn as_slice(&self) -> &[String] {
        self.0.as_ref()
    }
}

impl From<std::sync::Arc<[String]>> for RedactedAddressList {
    fn from(value: std::sync::Arc<[String]>) -> Self {
        Self(value)
    }
}

impl From<Vec<String>> for RedactedAddressList {
    fn from(value: Vec<String>) -> Self {
        Self(std::sync::Arc::from(value))
    }
}

impl AsRef<[String]> for RedactedAddressList {
    fn as_ref(&self) -> &[String] {
        self.as_slice()
    }
}

impl fmt::Debug for RedactedAddressList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddressList")
            .field(
                "addresses",
                &format_args!("<redacted>; len={}", self.0.len()),
            )
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct RedactedWalletTrackerBatch(Vec<(String, Result<WalletTrackerSnapshot, String>)>);

impl RedactedWalletTrackerBatch {
    pub(crate) fn into_vec(self) -> Vec<(String, Result<WalletTrackerSnapshot, String>)> {
        self.0
    }
}

impl From<Vec<(String, Result<WalletTrackerSnapshot, String>)>> for RedactedWalletTrackerBatch {
    fn from(value: Vec<(String, Result<WalletTrackerSnapshot, String>)>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for RedactedWalletTrackerBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletTrackerBatch")
            .field(
                "addresses",
                &format_args!("<redacted>; len={}", self.0.len()),
            )
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct RedactedJournalSnapshotRequest(journal::JournalTradeSnapshotRequest);

impl RedactedJournalSnapshotRequest {
    pub(crate) fn into_request(self) -> journal::JournalTradeSnapshotRequest {
        self.0
    }
}

impl From<journal::JournalTradeSnapshotRequest> for RedactedJournalSnapshotRequest {
    fn from(value: journal::JournalTradeSnapshotRequest) -> Self {
        Self(value)
    }
}

impl fmt::Debug for RedactedJournalSnapshotRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
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
    GhostWallet(RedactedAddress),
    ForgetGhostAccount(usize),
    DeleteSavedAccount(usize),
    SaveCredentials,
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    PaneClicked(pane_grid::Pane),
    SwitchBottomTab(BottomTab),
    OrderPriceChanged(RedactedOrderInput),
    SetMidPrice,
    OrderBookPriceSelected {
        id: OrderBookId,
        price: String,
    },
    OrderQuantityChanged(RedactedOrderInput),
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
    OpenWalletDetailsWindow(RedactedAddress),
    RefreshWalletDetails(window::Id),
    WalletDetailsLoaded(
        window::Id,
        RedactedAddress,
        ReadDataRequestContext,
        Box<Result<WalletDetailsData, String>>,
    ),
    WalletDetailsWsUpdate(Option<RedactedAddress>, Box<WsUserData>),
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
        account_key: RedactedAccountKey,
        address: RedactedAddress,
        result: Result<api::UserFillsPage, String>,
    },
    JournalRefresh,
    JournalClearCache,
    JournalEditStart(String, Option<String>),
    JournalEditCancel(String),
    JournalEditSave(String),
    JournalBufferChanged(String, bool, String),
    JournalCauseOfErrorChanged(String, String),
    JournalTagsChanged(String, String),
    JournalSelectTrade(String),
    JournalDeselectTrade,
    JournalSnapshotTimeframe(String, crate::timeframe::Timeframe),
    JournalSnapshotCoverageChanged(journal::JournalSnapshotCoverage),
    JournalFilterChanged(journal::JournalFilter),
    JournalSortChanged(journal::JournalSort),
    JournalPortfolioWindowChanged(PortfolioWindow),
    JournalChartRevealTick,
    JournalToggleAllAssets,
    JournalToggleAccountValueChart(bool),
    JournalToggleIncludeFeesInPnl,
    JournalSnapshotLoaded {
        account_key: RedactedAccountKey,
        address: RedactedAddress,
        request: RedactedJournalSnapshotRequest,
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
    WalletTrackerInputChanged(RedactedAddress),
    WalletTrackerLabelInputChanged(String),
    WalletTrackerAdd,
    WalletTrackerMute(RedactedAddress),
    WalletTrackerUnmute(RedactedAddress),
    WalletTrackerRemove(RedactedAddress),
    WalletTrackerLabelChanged(RedactedAddress, String),
    WalletTrackerRefresh,
    WalletTrackerRefreshDue,
    WalletTrackerRefreshOne(RedactedAddress),
    WalletTrackerRefreshOrdersDue,
    WalletTrackerRefreshOrders(RedactedAddress),
    WalletTrackerLoaded(
        RedactedAddress,
        ReadDataRequestContext,
        Box<Result<WalletTrackerSnapshot, String>>,
    ),
    WalletTrackerBatchLoaded(ReadDataRequestContext, RedactedWalletTrackerBatch),
    WalletTrackerOrdersLoaded(
        RedactedAddress,
        ReadDataRequestContext,
        Box<Result<usize, String>>,
    ),
    RefreshPortfolio,
    PortfolioLoaded(RedactedAddress, u64, Box<Result<PortfolioHistory, String>>),
    RefreshIncome,
    IncomeLoaded(RedactedAddress, u64, Box<Result<IncomeSnapshot, String>>),
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
    TelegramMediaLoaded(String, u64, String, u64, Box<Result<Vec<u8>, String>>),
    ToggleTelegramFastFeed,
    TelegramFeedDismissOnboarding,
    TelegramFeedShowOnboarding,
    ToggleTelegramFastAdvanced,
    TelegramFastCountryCodeChanged(String),
    TelegramFastEditNumber,
    TelegramFastApiIdChanged(SecretInput),
    TelegramFastApiHashChanged(SecretInput),
    TelegramFastPhoneChanged(RedactedPhoneInput),
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
    TelegramFeedRemoveChannel(RedactedTelegramChannelKey),
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
    EnterApplication,
    DismissToast(u64),
    ToastPositionChanged(config::ToastPosition),
    ToggleToastAnimations(bool),
    ToastAnimationTick,
    CopyToClipboard(RedactedClipboardText),
    WalletAddressActionsHovered(RedactedAddress),
    WalletAddressActionsExited(RedactedAddress),
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
        account_address: RedactedAddress,
        pending_indicator_id: Option<u64>,
        result: Box<Result<ExchangeResponse, String>>,
    },
    CancelOrderStatusLoaded {
        account_address: RedactedAddress,
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
    TwapDurationChanged(RedactedOrderInput),
    TwapSlicesChanged(RedactedOrderInput),
    TwapMinPriceChanged(RedactedOrderInput),
    TwapMaxPriceChanged(RedactedOrderInput),
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
    ChartSecondaryCandlesLoaded(CandleFetchRequest, Result<Vec<Candle>, String>),
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
    ChartSecondarySymbolSelected(ChartId, String),
    ChartSecondarySymbolRemoved(ChartId),
    ToggleChartInvert(ChartId),
    ToggleChartTradeMarkers(ChartId),
    ToggleChartHeaderCollapsed(ChartId),
    ToggleChartDrawingToolbar(ChartId),
    OpenDetachedChart(ChartId),
    ChartOpenEditor(ChartId),
    ChartCloseEditor(ChartId),
    ChartEditorSearchChanged(ChartId, String),
    ChartEditorSubmit(ChartId),
    ChartSecondaryOpenEditor(ChartId),
    ChartSecondaryCloseEditor(ChartId),
    ChartSecondaryEditorSearchChanged(ChartId, String),
    ChartSecondaryEditorSubmit(ChartId),
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
    QuickOrderQtyChanged(ChartId, RedactedOrderInput),
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
        account_address: RedactedAddress,
        coin: String,
        oid: u64,
        pending_indicator_id: Option<u64>,
        result: Box<Result<ExchangeResponse, String>>,
    },
    MoveOrderStatusLoaded {
        account_address: RedactedAddress,
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
    WalletAddressInputChanged(RedactedAddress),
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
        tracked_addresses: RedactedAddressList,
        message: crate::ws::HydromancerWsMessage,
    },
    ClearLiquidations,
    LiquidationFeedScrolled(iced::widget::scrollable::Viewport),
    ClearTrackedTrades,
    ConnectWallet,
    DisconnectWallet,
    AccountDataLoaded(
        RedactedAddress,
        AccountDataRequestContext,
        Box<Result<AccountData, String>>,
    ),
    RetryTwapReconciliationAccountData(RedactedAddress),
    RefreshAccountData,
    AccountRefreshBackoffElapsed(u64),
    AllMidsBootstrapLoaded(String, Result<HashMap<String, f64>, String>),
    WsUserDataUpdate(Option<RedactedAddress>, Box<WsUserData>),
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
    use super::{
        Message, RedactedOrderInput, RedactedPhoneInput, RedactedTelegramChannelKey, SecretInput,
        TelegramFastAuthMessageResult, TelegramFastAuthOutcome,
    };
    use crate::api::{ExchangeSymbol, ExchangeSymbolsPayload, MarketType, OutcomeSymbolInfo};
    use crate::chart_state::ChartSurfaceId;
    use crate::config::{ChartBackfillSource, MarketUniverseConfig, ReadDataProvider};
    use crate::order_execution::{
        OneShotPlacementContext, PendingLeverageUpdateContext, QuickOrderForm,
        QuickOrderQuantityProvenance, QuickOrderRecovery,
    };
    use crate::read_data_provider::{AccountDataRequestContext, ReadDataRequestContext};
    use crate::timeframe::Timeframe;
    use crate::ws::{HydromancerWsMessage, TrackedTradeEvent, WsUserData};

    #[test]
    fn secret_input_debug_redacts_value() {
        let rendered = format!("{:?}", SecretInput::from("super-secret"));

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("super-secret"));
    }

    #[test]
    fn order_input_debug_redacts_value() {
        let rendered = format!("{:?}", RedactedOrderInput::from("order-input-secret"));

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("order-input-secret"));
    }

    #[test]
    fn order_input_message_debug_redacts_value() {
        let messages = [
            Message::OrderPriceChanged("order-input-secret".into()),
            Message::OrderQuantityChanged("order-input-secret".into()),
            Message::TwapDurationChanged("order-input-secret".into()),
            Message::TwapSlicesChanged("order-input-secret".into()),
            Message::TwapMinPriceChanged("order-input-secret".into()),
            Message::TwapMaxPriceChanged("order-input-secret".into()),
            Message::QuickOrderQtyChanged(7, "order-input-secret".into()),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"));
            assert!(!rendered.contains("order-input-secret"));
        }
    }

    #[test]
    fn secret_bearing_message_debug_redacts_value() {
        let messages = [
            Message::EncryptedSecretPasswordChanged("sentinel-secret".into()),
            Message::EncryptedSecretConfirmChanged("sentinel-secret".into()),
            Message::TelegramFastApiIdChanged("sentinel-secret".into()),
            Message::TelegramFastApiHashChanged("sentinel-secret".into()),
            Message::TelegramFastCodeChanged("sentinel-secret".into()),
            Message::TelegramFastPasswordChanged("sentinel-secret".into()),
            Message::TelegramFastAuthResult(
                1,
                TelegramFastAuthMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::TelegramFastAuthResult(
                2,
                TelegramFastAuthMessageResult::new(Ok(TelegramFastAuthOutcome::SignedIn {
                    display_name: "sentinel-secret".to_string(),
                })),
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

    #[test]
    fn pii_bearing_message_debug_redacts_value() {
        let rendered = format!(
            "{:?}",
            Message::TelegramFastPhoneChanged(RedactedPhoneInput::from("+15555550123"))
        );

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("+15555550123"));
    }

    #[test]
    fn private_telegram_channel_message_debug_redacts_key() {
        let private_key = "private:1001234567890";
        let rendered = format!(
            "{:?}",
            Message::TelegramFeedRemoveChannel(RedactedTelegramChannelKey::from(private_key))
        );

        assert!(rendered.contains("<private>"));
        assert!(!rendered.contains(private_key));

        let public = format!(
            "{:?}",
            Message::TelegramFeedRemoveChannel(RedactedTelegramChannelKey::from("marketfeed"))
        );
        assert!(public.contains("marketfeed"));
    }

    #[test]
    fn symbols_loaded_message_debug_summarizes_exchange_metadata() {
        let message = Message::SymbolsLoaded(Ok(ExchangeSymbolsPayload {
            symbols: vec![ExchangeSymbol {
                key: "#660".to_string(),
                ticker: "#660".to_string(),
                category: "outcome".to_string(),
                display_name: Some("BTC above private threshold".to_string()),
                keywords: vec!["btc".to_string(), "private-threshold".to_string()],
                asset_index: 100_000_000,
                collateral_token: None,
                sz_decimals: 0,
                max_leverage: 1,
                only_isolated: true,
                market_type: MarketType::Outcome,
                outcome: Some(OutcomeSymbolInfo {
                    outcome_id: 66,
                    question_id: Some(12),
                    question_name: Some("Will BTC close above private threshold?".to_string()),
                    question_description: Some("Long raw outcome description".to_string()),
                    question_class: Some("priceBucket".to_string()),
                    question_underlying: Some("BTC".to_string()),
                    question_expiry: Some("20260520-0600".to_string()),
                    question_price_thresholds: vec!["75348.12".to_string()],
                    question_period: Some("1d".to_string()),
                    question_named_outcomes: vec![67, 68, 69],
                    question_settled_named_outcomes: Vec::new(),
                    question_fallback_outcome: Some(66),
                    bucket_index: Some(2),
                    is_question_fallback: false,
                    side_index: 0,
                    side_name: "Yes".to_string(),
                    outcome_name: "BTC above private threshold".to_string(),
                    description: "Outcome contract description".to_string(),
                    class: None,
                    underlying: None,
                    expiry: None,
                    target_price: Some("75348.12".to_string()),
                    period: None,
                    quote_symbol: "USDH".to_string(),
                    quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
                    encoding: 660,
                }),
            }],
            spot_meta_failed: false,
            outcome_meta_failed: false,
        }));

        let rendered = format!("{message:?}");

        assert!(rendered.contains("SymbolsLoaded"));
        assert!(rendered.contains("symbols_len: 1"));
        assert!(rendered.contains("outcome_count: 1"));
        assert!(!rendered.contains("private threshold"));
        assert!(!rendered.contains("Long raw outcome description"));
        assert!(!rendered.contains("75348.12"));
    }

    #[test]
    fn address_bearing_message_debug_redacts_values() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const ACCOUNT_KEY: &str = "account-key-sentinel";

        let read_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hyperliquid,
            read_data_provider_generation: 1,
            hydromancer_key_generation: 2,
        };
        let account_context = AccountDataRequestContext::connected_snapshot(read_context, 3);
        let snapshot_request = crate::journal::JournalTradeSnapshotRequest {
            account_key: Some(ACCOUNT_KEY.to_string()),
            address: ADDRESS.to_string(),
            trade_id: "trade-1".to_string(),
            coin: "HYPE".to_string(),
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: 1,
            hydromancer_key_generation: 2,
            coverage: crate::journal::JournalSnapshotCoverage::default(),
            timeframe: Timeframe::M1,
            ladder_index: 0,
            trade_start_ms: 100,
            trade_end_ms: 200,
            is_open: false,
            start_ms: 50,
            end_ms: 250,
        };

        let messages = vec![
            Message::GhostWallet(ADDRESS.into()),
            Message::OpenWalletDetailsWindow(ADDRESS.into()),
            Message::WalletDetailsLoaded(
                iced::window::Id::unique(),
                ADDRESS.into(),
                read_context,
                Box::new(Err("details failed".to_string())),
            ),
            Message::WalletDetailsWsUpdate(
                Some(ADDRESS.into()),
                Box::new(WsUserData::Lagged { skipped: 1 }),
            ),
            Message::JournalFillsLoaded {
                request_id: 1,
                account_key: Some(ACCOUNT_KEY.to_string()).into(),
                address: ADDRESS.into(),
                result: Err("fills failed".to_string()),
            },
            Message::JournalSnapshotLoaded {
                account_key: Some(ACCOUNT_KEY.to_string()).into(),
                address: ADDRESS.into(),
                request: snapshot_request.into(),
                result: Err("snapshot failed".to_string()),
            },
            Message::WalletTrackerInputChanged(ADDRESS.into()),
            Message::WalletTrackerMute(ADDRESS.into()),
            Message::WalletTrackerUnmute(ADDRESS.into()),
            Message::WalletTrackerRemove(ADDRESS.into()),
            Message::WalletTrackerLabelChanged(ADDRESS.into(), "desk".to_string()),
            Message::WalletTrackerRefreshOne(ADDRESS.into()),
            Message::WalletTrackerRefreshOrders(ADDRESS.into()),
            Message::WalletTrackerLoaded(
                ADDRESS.into(),
                read_context,
                Box::new(Err("tracker failed".to_string())),
            ),
            Message::WalletTrackerBatchLoaded(
                read_context,
                vec![(ADDRESS.to_string(), Err("batch failed".to_string()))].into(),
            ),
            Message::WalletTrackerOrdersLoaded(
                ADDRESS.into(),
                read_context,
                Box::new(Err("orders failed".to_string())),
            ),
            Message::PortfolioLoaded(
                ADDRESS.into(),
                1,
                Box::new(Err("portfolio failed".to_string())),
            ),
            Message::IncomeLoaded(
                ADDRESS.into(),
                1,
                Box::new(Err("income failed".to_string())),
            ),
            Message::CopyToClipboard(ADDRESS.into()),
            Message::WalletAddressActionsHovered(ADDRESS.into()),
            Message::WalletAddressActionsExited(ADDRESS.into()),
            Message::QuickOrderResult {
                pending_indicator_id: None,
                context: OneShotPlacementContext {
                    account_address: ADDRESS.to_string(),
                    cloid: "0x00000000000000000000000000000000".to_string(),
                    surface: crate::order_execution::OrderSurface::QuickOrder,
                    symbol_key: "HYPE".to_string(),
                    order_kind: crate::signing::ExchangeOrderKind::Limit,
                },
                recovery: Some(QuickOrderRecovery {
                    chart_id: 1,
                    form: QuickOrderForm {
                        price: 100.0,
                        quantity: "1".to_string(),
                        quantity_is_usd: false,
                        percentage: 25.0,
                        quantity_provenance: Some(QuickOrderQuantityProvenance {
                            account_address: ADDRESS.to_string(),
                            account_data_revision: 1,
                            symbol_key: "HYPE".to_string(),
                            quantity_is_usd: false,
                            percentage: 25.0,
                            is_limit: true,
                            reference_price: Some(100.0),
                            reduce_only: false,
                            market_universe: MarketUniverseConfig::default(),
                        }),
                        is_limit: true,
                        click_x: 0.0,
                        click_y: 0.0,
                        chart_w: 100.0,
                        chart_h: 100.0,
                    },
                    surface_id: Some(ChartSurfaceId::Docked(1)),
                }),
                result: Box::new(Err("quick failed".to_string())),
            },
            Message::OrderLeverageResult {
                context: PendingLeverageUpdateContext {
                    address: ADDRESS.to_string(),
                    symbol_key: "HYPE".to_string(),
                    display: "HYPE".to_string(),
                    asset: 42,
                    dex: None,
                    is_cross: true,
                    leverage: 3,
                },
                result: Box::new(Err("leverage failed".to_string())),
            },
            Message::CancelResult {
                account_address: ADDRESS.into(),
                pending_indicator_id: None,
                result: Box::new(Err("cancel failed".to_string())),
            },
            Message::CancelOrderStatusLoaded {
                account_address: ADDRESS.into(),
                oid: 42,
                symbol: "HYPE".to_string(),
                result: Box::new(Err("status failed".to_string())),
            },
            Message::MoveOrderModifyResult {
                account_address: ADDRESS.into(),
                coin: "HYPE".to_string(),
                oid: 42,
                pending_indicator_id: None,
                result: Box::new(Err("modify failed".to_string())),
            },
            Message::MoveOrderStatusLoaded {
                account_address: ADDRESS.into(),
                coin: "HYPE".to_string(),
                oid: 42,
                result: Box::new(Err("move status failed".to_string())),
            },
            Message::WalletAddressInputChanged(ADDRESS.into()),
            Message::WsHydromancerTrackedTrades {
                hydromancer_key_generation: 1,
                reconnect_nonce: 2,
                tracked_addresses: std::sync::Arc::<[String]>::from(vec![ADDRESS.to_string()])
                    .into(),
                message: HydromancerWsMessage::TrackedTrade(TrackedTradeEvent {
                    address: ADDRESS.to_string(),
                    coin: "HYPE".to_string(),
                    price: 10.0,
                    size: 1.0,
                    is_buy: true,
                    time_ms: 100,
                    dir: "Open Long".to_string(),
                    start_position: Some(0.0),
                    closed_pnl: 0.0,
                    fee: 0.01,
                    fee_token: "USDC".to_string(),
                    tid: Some(123),
                    hash: "0xabc".to_string(),
                    oid: Some(456),
                    tx_index: 7,
                }),
            },
            Message::AccountDataLoaded(
                ADDRESS.into(),
                account_context,
                Box::new(Err("account failed".to_string())),
            ),
            Message::RetryTwapReconciliationAccountData(ADDRESS.into()),
            Message::WsUserDataUpdate(
                Some(ADDRESS.into()),
                Box::new(WsUserData::Lagged { skipped: 1 }),
            ),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(ADDRESS), "{rendered}");
            assert!(!rendered.contains(ACCOUNT_KEY), "{rendered}");
        }
    }
}
