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
use crate::openrouter_api::OpenRouterKeyStatus;
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
use crate::schwab::{SchwabAccountsSnapshot, SchwabOAuthTokenRefresh};
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
use crate::wallet_cluster_state::WalletClusterCloseSide;
use crate::ws::{WsUserData, WsUserDataStreamParams};
use crate::x_feed::{
    XAuthenticatedUser, XFeedId, XFeedPage, XFeedRequestError, XFeedSource, XFeedSourceOption,
    XListsFetchOutcome, XOAuthTokenRefresh,
};
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
pub(crate) struct RedactedWalletLabel(String);

impl RedactedWalletLabel {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedWalletLabel {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedWalletLabel {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedWalletLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WalletLabel(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedAccountLabel(String);

impl RedactedAccountLabel {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedAccountLabel {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedAccountLabel {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedAccountLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountLabel(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedAccountProfileId(String);

impl RedactedAccountProfileId {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedAccountProfileId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedAccountProfileId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedAccountProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountProfileId(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedWalletClusterId(String);

impl RedactedWalletClusterId {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedWalletClusterId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedWalletClusterId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedWalletClusterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WalletClusterId(<redacted>)")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RedactedWalletClusterName(String);

impl RedactedWalletClusterName {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedWalletClusterName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedWalletClusterName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedWalletClusterName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WalletClusterName(<redacted>)")
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

/// Exact financial order value carried through the transient message boundary.
///
/// Update handlers recover the original value without conversion. Derived
/// `Message::Debug` output cannot expose a price, percentage, fraction, or
/// nested preset payload.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct RedactedOrderValue<T>(T);

impl<T> RedactedOrderValue<T> {
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for RedactedOrderValue<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for RedactedOrderValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OrderValue(<redacted>)")
    }
}

/// Exchange order ID carried through the transient message boundary.
///
/// The exact value remains available to update handlers, while the derived
/// `Message::Debug` path receives only this redacted representation.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) struct RedactedOrderId(u64);

impl RedactedOrderId {
    pub(crate) fn into_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for RedactedOrderId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Debug for RedactedOrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OrderId(<redacted>)")
    }
}

/// Client order ID carried through the transient message boundary.
///
/// The exact value remains available to update handlers, while the derived
/// `Message::Debug` path receives only this redacted representation.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct RedactedClientOrderId(String);

impl RedactedClientOrderId {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedClientOrderId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedClientOrderId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedClientOrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClientOrderId(<redacted>)")
    }
}

/// Exchange symbol carried through an order-lifecycle message.
///
/// The exact value remains available to update handlers, while the derived
/// `Message::Debug` path receives only this redacted representation.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct RedactedOrderSymbol(String);

impl RedactedOrderSymbol {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedOrderSymbol {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedOrderSymbol {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedOrderSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OrderSymbol(<redacted>)")
    }
}

/// Persisted advanced-order history identity carried through navigation.
///
/// The exact value remains available to the history-window handler, while the
/// derived `Message::Debug` path cannot expose the embedded account identity.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct RedactedAdvancedOrderHistoryId(String);

impl RedactedAdvancedOrderHistoryId {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for RedactedAdvancedOrderHistoryId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for RedactedAdvancedOrderHistoryId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for RedactedAdvancedOrderHistoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AdvancedOrderHistoryId(<redacted>)")
    }
}

/// Exact account task result carried through the Elm message boundary.
///
/// Update handlers recover the original value. Generic message diagnostics
/// expose only success/error shape and never traverse a nested account payload
/// or external error string.
#[derive(Clone)]
pub(crate) struct RedactedAccountMessageResult<T>(Box<Result<T, String>>);

impl<T> RedactedAccountMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        *self.0
    }
}

impl<T> From<Result<T, String>> for RedactedAccountMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(Box::new(value))
    }
}

impl<T> fmt::Debug for RedactedAccountMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact HyperDash positioning result carried through the Elm message boundary.
///
/// Update handlers recover the original wallet-level positions, deltas, or
/// external error. Generic message diagnostics expose only success/error shape
/// and never traverse the account-identifying payload.
#[derive(Clone)]
pub(crate) struct RedactedPositioningMessageResult<T>(Box<Result<T, String>>);

impl<T> RedactedPositioningMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        *self.0
    }
}

impl<T> From<Result<T, String>> for RedactedPositioningMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(Box::new(value))
    }
}

impl<T> fmt::Debug for RedactedPositioningMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact public-market HyperDash result carried through the Elm message boundary.
///
/// Standalone liquidation and heatmap models remain structurally diagnosable.
/// Generic message diagnostics expose only success/error shape, however, and
/// never traverse a potentially large payload or pre-handler external error.
#[derive(Clone)]
pub(crate) struct RedactedHyperdashMarketMessageResult<T>(Box<Result<T, String>>);

impl<T> RedactedHyperdashMarketMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        *self.0
    }
}

impl<T> From<Result<T, String>> for RedactedHyperdashMarketMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(Box::new(value))
    }
}

impl<T> fmt::Debug for RedactedHyperdashMarketMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact public market refresh result carried through the Elm message boundary.
///
/// Request IDs, requested symbols, and timestamps remain visible correlation
/// context. Generic message diagnostics expose only success/error shape and
/// never traverse a potentially large payload or pre-handler external error.
#[derive(Clone)]
pub(crate) struct RedactedPublicMarketMessageResult<T>(Box<Result<T, String>>);

impl<T> RedactedPublicMarketMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        *self.0
    }
}

impl<T> From<Result<T, String>> for RedactedPublicMarketMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(Box::new(value))
    }
}

impl<T> fmt::Debug for RedactedPublicMarketMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact trading-journal task result carried through the Elm message boundary.
///
/// Update handlers recover the original fills, candles, or external error.
/// Generic message diagnostics expose only success/error shape and never
/// traverse account activity or snapshot payloads.
#[derive(Clone)]
pub(crate) struct RedactedJournalMessageResult<T>(Result<T, String>);

impl<T> RedactedJournalMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        self.0
    }
}

impl<T> From<Result<T, String>> for RedactedJournalMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for RedactedJournalMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact saved-layout I/O result carried through the Elm message boundary.
///
/// Update handlers recover the original layout or external error. Generic
/// message diagnostics expose only success/error shape.
#[derive(Clone)]
pub(crate) struct RedactedLayoutMessageResult<T>(Result<T, String>);

impl<T> RedactedLayoutMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        self.0
    }
}

impl<T> From<Result<T, String>> for RedactedLayoutMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for RedactedLayoutMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact wallet-label file result carried through the Elm message boundary.
///
/// Update handlers recover the original export or external error. Generic
/// message diagnostics expose only success/error shape and never traverse
/// account identity metadata or unsanitized file/parse errors.
#[derive(Clone)]
pub(crate) struct RedactedWalletLabelsMessageResult<T>(Result<T, String>);

impl<T> RedactedWalletLabelsMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        self.0
    }
}

impl<T> From<Result<T, String>> for RedactedWalletLabelsMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for RedactedWalletLabelsMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact PnL-card export result carried through the Elm message boundary.
///
/// Update handlers recover the original saved path or external error. Generic
/// message diagnostics expose only success/error shape.
#[derive(Clone)]
pub(crate) struct RedactedPnlCardMessageResult<T>(Result<T, String>);

impl<T> RedactedPnlCardMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        self.0
    }
}

impl<T> From<Result<T, String>> for RedactedPnlCardMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for RedactedPnlCardMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

/// Exact order task result carried through the Elm message boundary.
///
/// Update handlers recover the original value. Generic message diagnostics
/// expose only success/error shape and never traverse a nested response or
/// external error string.
#[derive(Clone)]
pub(crate) struct RedactedOrderMessageResult<T>(Box<Result<T, String>>);

impl<T> RedactedOrderMessageResult<T> {
    pub(crate) fn into_result(self) -> Result<T, String> {
        *self.0
    }
}

impl<T> From<Result<T, String>> for RedactedOrderMessageResult<T> {
    fn from(value: Result<T, String>) -> Self {
        Self(Box::new(value))
    }
}

impl<T> fmt::Debug for RedactedOrderMessageResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(_) => f.write_str("Ok(<redacted>)"),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
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

#[derive(Clone)]
pub(crate) struct XAuthContextMessageResult(
    Box<Result<(XAuthenticatedUser, XListsFetchOutcome), String>>,
);

impl XAuthContextMessageResult {
    pub(crate) fn new(result: Result<(XAuthenticatedUser, XListsFetchOutcome), String>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<(XAuthenticatedUser, XListsFetchOutcome), String> {
        *self.0
    }
}

impl fmt::Debug for XAuthContextMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok((user, outcome)) => f
                .debug_struct("XAuthContextMessageResult")
                .field("user", user)
                .field("lists", &outcome.lists.len())
                .field("unavailable_sources", &outcome.unavailable_sources)
                .finish(),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct XAccessTokenRefreshMessageResult(Box<Result<XOAuthTokenRefresh, String>>);

impl XAccessTokenRefreshMessageResult {
    pub(crate) fn new(result: Result<XOAuthTokenRefresh, String>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<XOAuthTokenRefresh, String> {
        *self.0
    }
}

impl fmt::Debug for XAccessTokenRefreshMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(refresh) => f
                .debug_struct("XAccessTokenRefreshMessageResult")
                .field("refresh", refresh)
                .finish(),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct XListsMessageResult(Box<Result<XListsFetchOutcome, String>>);

impl XListsMessageResult {
    pub(crate) fn new(result: Result<XListsFetchOutcome, String>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<XListsFetchOutcome, String> {
        *self.0
    }
}

impl fmt::Debug for XListsMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(outcome) => f
                .debug_struct("XListsMessageResult")
                .field("lists", &outcome.lists.len())
                .field("unavailable_sources", &outcome.unavailable_sources)
                .finish(),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct XFeedPageMessageResult(Box<Result<XFeedPage, XFeedRequestError>>);

impl XFeedPageMessageResult {
    pub(crate) fn new(result: Result<XFeedPage, XFeedRequestError>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<XFeedPage, XFeedRequestError> {
        *self.0
    }
}

impl fmt::Debug for XFeedPageMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(page) => page.fmt(f),
            Err(error) => f
                .debug_tuple("XFeedPageMessageResult")
                .field(error)
                .finish(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct XProfileImageMessageResult(Box<Result<Vec<u8>, String>>);

impl XProfileImageMessageResult {
    pub(crate) fn new(result: Result<Vec<u8>, String>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<Vec<u8>, String> {
        *self.0
    }
}

impl fmt::Debug for XProfileImageMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(bytes) => f
                .debug_struct("XProfileImageMessageResult")
                .field("bytes", &bytes.len())
                .finish(),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct SchwabTokenRefreshMessageResult(Box<Result<SchwabOAuthTokenRefresh, String>>);

impl SchwabTokenRefreshMessageResult {
    pub(crate) fn new(result: Result<SchwabOAuthTokenRefresh, String>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<SchwabOAuthTokenRefresh, String> {
        *self.0
    }
}

impl fmt::Debug for SchwabTokenRefreshMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(refresh) => f
                .debug_struct("SchwabTokenRefreshMessageResult")
                .field("refresh", refresh)
                .finish(),
            Err(_) => f.write_str("Err(<redacted>)"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct SchwabAccountsMessageResult(Box<Result<SchwabAccountsSnapshot, String>>);

impl SchwabAccountsMessageResult {
    pub(crate) fn new(result: Result<SchwabAccountsSnapshot, String>) -> Self {
        Self(Box::new(result))
    }

    pub(crate) fn into_result(self) -> Result<SchwabAccountsSnapshot, String> {
        *self.0
    }
}

impl fmt::Debug for SchwabAccountsMessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            Ok(snapshot) => f
                .debug_struct("SchwabAccountsMessageResult")
                .field("snapshot", snapshot)
                .finish(),
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
    LayoutImported(RedactedLayoutMessageResult<config::SavedLayout>),
    LayoutExported(RedactedLayoutMessageResult<()>),
    ExportWalletLabels,
    ImportWalletLabels,
    WalletLabelsExported(RedactedWalletLabelsMessageResult<()>),
    WalletLabelsImported(RedactedWalletLabelsMessageResult<config::WalletLabelsExport>),
    LayoutInputChanged(String),
    LiveWatchlistSearchChanged(LiveWatchlistId, String),
    LiveWatchlistContextsLoaded(
        u64,
        Vec<String>,
        u64,
        RedactedPublicMarketMessageResult<crate::api::WatchlistContextsResponse>,
    ),
    LiveWatchlistHistoryLoaded(
        u64,
        Vec<String>,
        u64,
        RedactedPublicMarketMessageResult<HashMap<String, (f64, f64, f64)>>,
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
    PositioningInfoEntryMinChanged(PositioningInfoId, String),
    PositioningInfoEntryMaxChanged(PositioningInfoId, String),
    ApplyPositioningInfoEntryRange(PositioningInfoId),
    PositioningInfoChangeTimeframeChanged(PositioningInfoId, PositioningInfoChangeTimeframe),
    ClearPositioningInfoFilters(PositioningInfoId),
    RefreshPositioningInfoPane(PositioningInfoId),
    RefreshPositioningInfo,
    PositioningInfoWsAssetCtxUpdate(String, MarketDataSourceContext, AssetContext),
    PositioningInfoWsAssetCtxLagged(String, MarketDataSourceContext, u64),
    PositioningInfoLoaded(
        String,
        u64,
        RedactedPositioningMessageResult<crate::hyperdash_api::TickerPositions>,
    ),
    PositioningInfoChangeLoaded(
        String,
        u64,
        RedactedPositioningMessageResult<crate::hyperdash_api::PerpDeltas>,
    ),
    AddOrderBookPane,
    AddAdvancedOrdersPane,
    PositionsSortChanged(PositionsSortColumn),

    ToggleAccountPicker,
    AccountPickerSelected(usize),
    AccountPickerRenameToggled(usize),
    AccountPickerLabelChanged(usize, RedactedAccountLabel),
    OpenAddAccountWindow,
    AddAccountNameChanged(RedactedAccountLabel),
    AddAccountAddressChanged(RedactedAddress),
    AddAccountKeyChanged(SecretInput),
    AddAccountSwitchToggled(bool),
    AddAccountSubmit,
    AddAccountCancel,
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
        price: RedactedOrderInput,
    },
    OrderQuantityChanged(RedactedOrderInput),
    SetOrderKind(OrderKind),
    ToggleOrderDenomination,
    OrderPercentageChanged(RedactedOrderValue<f32>),
    PrefillOutcomeSell(RedactedOrderSymbol),
    ToggleReduceOnly,
    ToggleOrderLeverageDropdown,
    OrderLeverageInputChanged(RedactedOrderInput),
    SetOrderLeverageCross(bool),
    SubmitOrderLeverage(OrderLeverageSubmissionSnapshot),
    OrderLeverageResult {
        context: PendingLeverageUpdateContext,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    TogglePresetsMenu,
    TogglePresetCurrency,
    TogglePresetEditMode,
    SetAddWidgetPlacement(AddWidgetPlacement),
    EditPresetStart(crate::signing::OrderKind, usize, RedactedOrderInput),
    EditPresetChanged(RedactedOrderInput),
    EditPresetSave(crate::signing::OrderKind, usize),
    ExecutePreset(
        crate::signing::OrderKind,
        RedactedOrderValue<crate::config::OrderPreset>,
        bool,
    ),
    ToggleFavourite(String),
    ToggleTickerTape,
    TickerTapeTick,
    TickerTapeRefreshTick,
    TickerTapeContextsLoaded(
        u64,
        Vec<String>,
        u64,
        RedactedPublicMarketMessageResult<crate::api::WatchlistContextsResponse>,
    ),
    // Add widget menu
    ToggleAddWidgetMenu,
    ToggleLayoutMenu,
    ToggleMacroMenu(ChartId),
    ToggleMacroIndicator(ChartId, String),
    ToggleChartEarningsMarkers(ChartId),
    ChartEarningsEventsLoaded(String, u64, Box<Result<Vec<api::SecEarningsEvent>, String>>),
    ChartEarningsFilingSummaryLoaded(String, u64, Box<Result<api::SecFilingSummary, String>>),
    OpenChartEarningsFiling(ChartId, ChartSurfaceId, u64),
    ChartEarningsFilingOpenResult(Result<(), String>),
    CloseAllMenus,
    AddPositionsHistoryPane,
    AddPortfolioPane,
    AddIncomePane,
    AddComparisonChart,
    AddPairRatioChart,
    OpenSettingsWindow,
    OpenIntegrationsSettings,
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
        RedactedPublicMarketMessageResult<crate::api::WatchlistContextsResponse>,
    ),
    ScreenerHistoryLoaded(
        u64,
        Vec<String>,
        u64,
        RedactedPublicMarketMessageResult<HashMap<String, (f64, f64)>>,
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
    AddXFeedPane,
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
    ToggleChartGradientBackground(bool),
    ChartGradientContrastChanged(f32),
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
    ToggleHydromancerRealtimePositionPnl(bool),
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
    MarketSlippageInputChanged(RedactedOrderInput),
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
    OpenWalletClustersWindow,
    WalletClusterNameInputChanged(RedactedWalletClusterName),
    WalletClusterCreate,
    WalletClusterSelected(RedactedWalletClusterId),
    WalletClusterRenamed(RedactedWalletClusterId, RedactedWalletClusterName),
    WalletClusterDeleted(RedactedWalletClusterId),
    WalletClusterAddMember(RedactedAccountProfileId),
    WalletClusterRemoveMember(RedactedWalletClusterId, RedactedAccountKey),
    WalletClusterMemberWeightChanged(
        RedactedWalletClusterId,
        RedactedAccountKey,
        RedactedOrderInput,
    ),
    WalletClusterRefresh,
    WalletClusterMemberLoaded(
        RedactedWalletClusterId,
        RedactedAccountKey,
        RedactedAddress,
        ReadDataRequestContext,
        RedactedAccountMessageResult<WalletDetailsData>,
    ),
    WalletClusterWsUpdate(
        WsUserDataStreamParams,
        Option<RedactedAddress>,
        Box<WsUserData>,
    ),
    WalletClusterOrderPriceChanged(RedactedOrderInput),
    WalletClusterOrderQuantityChanged(RedactedOrderInput),
    WalletClusterToggleOrderDenomination,
    WalletClusterSetOrderKind(OrderKind),
    WalletClusterToggleReduceOnly,
    WalletClusterSetMidPrice,
    WalletClusterSubmitOrder {
        is_buy: bool,
    },
    WalletClusterClosePosition {
        symbol: RedactedOrderSymbol,
        side: WalletClusterCloseSide,
        fraction: RedactedOrderValue<f64>,
        use_market: bool,
    },
    WalletClusterOrderResult {
        execution_id: u64,
        member_key: RedactedAccountKey,
        context: OneShotPlacementContext,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    WalletClusterOrderStatusLoaded {
        execution_id: u64,
        member_key: RedactedAccountKey,
        context: OneShotPlacementContext,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
    },
    OpenWalletDetailsWindow(RedactedAddress),
    RefreshWalletDetails(window::Id),
    WalletDetailsLoaded(
        window::Id,
        RedactedAddress,
        ReadDataRequestContext,
        RedactedAccountMessageResult<WalletDetailsData>,
    ),
    WalletDetailsWsUpdate(
        WsUserDataStreamParams,
        Option<RedactedAddress>,
        Box<WsUserData>,
    ),
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
        result: RedactedJournalMessageResult<api::UserFillsPage>,
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
        result: RedactedJournalMessageResult<Vec<Candle>>,
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
    WalletTrackerLabelInputChanged(RedactedWalletLabel),
    WalletTrackerAdd,
    WalletTrackerMute(RedactedAddress),
    WalletTrackerUnmute(RedactedAddress),
    WalletTrackerRemove(RedactedAddress),
    WalletTrackerLabelChanged(RedactedAddress, RedactedWalletLabel),
    WalletTrackerRefresh,
    WalletTrackerRefreshDue,
    WalletTrackerRefreshOne(RedactedAddress),
    WalletTrackerRefreshOrdersDue,
    WalletTrackerRefreshOrders(RedactedAddress),
    WalletTrackerLoaded(
        RedactedAddress,
        ReadDataRequestContext,
        RedactedAccountMessageResult<WalletTrackerSnapshot>,
    ),
    WalletTrackerBatchLoaded(ReadDataRequestContext, RedactedWalletTrackerBatch),
    WalletTrackerOrdersLoaded(
        RedactedAddress,
        ReadDataRequestContext,
        RedactedAccountMessageResult<usize>,
    ),
    RefreshPortfolio,
    PortfolioLoaded(
        RedactedAddress,
        u64,
        RedactedAccountMessageResult<PortfolioHistory>,
    ),
    RefreshIncome,
    IncomeLoaded(
        RedactedAddress,
        u64,
        RedactedAccountMessageResult<IncomeSnapshot>,
    ),
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
    XFeedAccessTokenChanged(SecretInput),
    XFeedOAuthClientIdChanged(SecretInput),
    XFeedRefreshTokenChanged(SecretInput),
    XFeedConnect,
    XAccessTokenRefreshed(u64, XAccessTokenRefreshMessageResult),
    XFeedAuthLoaded(u64, XAuthContextMessageResult),
    XFeedClearAccessToken,
    XFeedListsRefresh,
    XFeedListsLoaded(u64, XListsMessageResult),
    XFeedSourceSelected(XFeedId, XFeedSourceOption),
    RefreshXFeed(XFeedId),
    XFeedRefreshTick,
    XFeedLoaded(XFeedSource, u64, XFeedPageMessageResult),
    XProfileImageLoaded(u64, XProfileImageMessageResult),
    SchwabClientIdChanged(SecretInput),
    SchwabClientSecretChanged(SecretInput),
    SchwabAccessTokenChanged(SecretInput),
    SchwabRefreshTokenChanged(SecretInput),
    SchwabConnect,
    SchwabAccessTokenRefreshed(u64, SchwabTokenRefreshMessageResult),
    SchwabAccountsRefresh,
    SchwabAccountsLoaded(u64, SchwabAccountsMessageResult),
    SchwabAccountPickerSelected(RedactedAccountKey),
    SchwabClearCredentials,
    SchwabTokenRefreshTick,
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
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    DismissOrderStatus,
    CancelOrder {
        coin: RedactedOrderSymbol,
        oid: RedactedOrderId,
    },
    CancelResult {
        request_id: u64,
        account_address: RedactedAddress,
        pending_indicator_id: Option<u64>,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    CancelOrderStatusLoaded {
        request_id: u64,
        account_address: RedactedAddress,
        oid: RedactedOrderId,
        symbol: RedactedOrderSymbol,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
    },
    ToggleCloseMenu(RedactedOrderSymbol),
    ToggleHiddenPosition(RedactedOrderSymbol),
    ToggleShowHiddenPositions,
    OpenPnlCard(PnlCardTarget),
    SetPnlCardDisplayMode(window::Id, PnlCardDisplayMode),
    SetPnlCardPercentMode(window::Id, PnlCardPercentMode),
    TogglePnlCardPricePrivacy(window::Id, bool),
    TogglePnlCardPositionSize(window::Id, bool),
    CopyPnlCard(window::Id),
    PnlCardCopied(RedactedPnlCardMessageResult<()>),
    SavePnlCard(window::Id),
    PnlCardSaved(RedactedPnlCardMessageResult<Option<PathBuf>>),
    ClosePosition {
        coin: RedactedOrderSymbol,
        fraction: RedactedOrderValue<f64>,
        use_market: bool,
    },
    ClosePositionResult {
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    NukePositions,
    NukeResult {
        execution_id: u64,
        context: OneShotPlacementContext,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    NukePlacementStatusLoaded {
        execution_id: u64,
        context: OneShotPlacementContext,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
    },
    OneShotPlacementStatusLoaded {
        request_id: u64,
        context: OneShotPlacementContext,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
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
        coin: RedactedOrderSymbol,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        book: OrderBook,
    },
    TwapBookLagged {
        twap_id: u64,
        coin: RedactedOrderSymbol,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        skipped: u64,
    },
    TwapSliceResult {
        twap_id: u64,
        slice_index: u32,
        retry_count: u32,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    TwapUnexpectedCancelResult {
        twap_id: u64,
        oid: Option<RedactedOrderId>,
        cloid: Option<RedactedClientOrderId>,
        attempt: u32,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    TwapUnexpectedCancelRetryDue {
        twap_id: u64,
        oid: Option<RedactedOrderId>,
        cloid: Option<RedactedClientOrderId>,
        attempt: u32,
    },
    TwapOrderStatusLoaded {
        twap_id: u64,
        cloid: RedactedClientOrderId,
        attempt: u32,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
    },
    OpenTwapDetails(u64),
    OpenAdvancedOrderHistory(RedactedAdvancedOrderHistoryId),
    ChaseInitialBookLoaded {
        chase_id: u64,
        result: RedactedOrderMessageResult<OrderBook>,
    },
    ChaseBookUpdate {
        chase_id: u64,
        coin: RedactedOrderSymbol,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        book: OrderBook,
    },
    ChaseBookLagged {
        chase_id: u64,
        coin: RedactedOrderSymbol,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        skipped: u64,
    },
    ChaseRepriceTick,
    ChasePlaceResult {
        chase_id: u64,
        place_attempt: u32,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    ChaseModifyResult {
        chase_id: u64,
        oid: RedactedOrderId,
        reprice_count: u32,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    ChaseCancelResult {
        chase_id: u64,
        oid: RedactedOrderId,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    ChaseOrderStatusLoaded {
        chase_id: u64,
        cloid: RedactedClientOrderId,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
    },
    ChaseOrderOidStatusLoaded {
        chase_id: u64,
        oid: RedactedOrderId,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
    },
    ChaseRestingOrder {
        coin: RedactedOrderSymbol,
        oid: RedactedOrderId,
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
    ChartHoverStateChanged(
        ChartId,
        ChartSurfaceId,
        Option<RedactedOrderId>,
        bool,
        Option<u64>,
    ),
    ChartOrderCancelHoverAnimationTick,
    ChartEarningsMarkerHoverAnimationTick,
    ChartWsAssetCtxUpdate(ChartId, String, MarketDataSourceContext, AssetContext),
    ChartWsAssetCtxLagged(ChartId, String, MarketDataSourceContext, u64),
    /// Result of the REST `metaAndAssetCtxs` fallback fetch for a chart symbol
    /// (chart id, symbol the fetch was issued for, fetched context).
    ChartAssetContextRestFetched(ChartId, String, Result<Option<AssetContext>, String>),
    /// Result of one coalesced `spotMetaAndAssetCtxs` request for every due
    /// spot chart (targets, contexts keyed by symbol).
    ChartSpotAssetContextsRestFetched(
        Vec<(ChartId, String)>,
        Result<Vec<(String, AssetContext)>, String>,
    ),
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
    OpenQuickOrder(
        ChartId,
        ChartSurfaceId,
        RedactedOrderValue<f64>,
        f32,
        f32,
        f32,
        f32,
    ),
    QuickOrderQtyChanged(ChartId, RedactedOrderInput),
    QuickOrderPercentageChanged(ChartId, RedactedOrderValue<f32>),
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
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    SubmitHudOrder(HudOrderRequest),
    HudOrderResult {
        pending_indicator_id: Option<u64>,
        inflight_id: Option<u64>,
        context: OneShotPlacementContext,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    EscapePressed(window::Id),
    // Order drag-to-move (from chart canvas)
    MoveOrderDragStarted {
        coin: RedactedOrderSymbol,
        oid: RedactedOrderId,
    },
    MoveOrder {
        coin: RedactedOrderSymbol,
        oid: RedactedOrderId,
        new_price: RedactedOrderValue<f64>,
    },
    MoveOrderModifyResult {
        request_id: u64,
        account_address: RedactedAddress,
        coin: RedactedOrderSymbol,
        oid: RedactedOrderId,
        pending_indicator_id: Option<u64>,
        result: RedactedOrderMessageResult<ExchangeResponse>,
    },
    MoveOrderStatusLoaded {
        request_id: u64,
        account_address: RedactedAddress,
        coin: RedactedOrderSymbol,
        oid: RedactedOrderId,
        result: RedactedOrderMessageResult<api::OrderStatusResult>,
    },
    // Global messages
    SymbolsLoaded(
        u64,
        RedactedPublicMarketMessageResult<api::ExchangeSymbolsPayload>,
    ),
    ExchangeSymbolsRefreshTick,
    SymbolSearchChanged(String),
    SymbolSearchSortChanged(SymbolSearchSortMode),
    SymbolSearchMarketFilterChanged(SymbolSearchMarketFilter),
    SymbolSearchHip3DexFilterChanged(String),
    SymbolSearchContextsLoaded(
        u64,
        Vec<String>,
        u64,
        RedactedPublicMarketMessageResult<crate::api::WatchlistContextsResponse>,
    ),
    OutcomeSearchChanged(String),
    OutcomeMarketGroupToggled(String),
    OutcomeVolumesLoaded(
        u64,
        Vec<String>,
        RedactedPublicMarketMessageResult<HashMap<String, crate::api::OutcomeVolume24h>>,
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
    PositionPnlWsBookUpdate {
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        book: OrderBook,
    },
    PositionPnlWsBookLagged {
        coin: String,
        sigfigs: (Option<u8>, Option<u8>),
        source_context: MarketDataSourceContext,
        skipped: u64,
    },
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
        RedactedAccountMessageResult<AccountData>,
    ),
    RetryTwapReconciliationAccountData(RedactedAddress),
    RefreshAccountData,
    AccountRefreshBackoffElapsed(u64),
    AllMidsBootstrapLoaded(String, Result<HashMap<String, f64>, String>),
    WsUserDataUpdate(
        WsUserDataStreamParams,
        Option<RedactedAddress>,
        Box<WsUserData>,
    ),
    // HyperDash liquidation heatmap
    HyperdashKeyInputChanged(SecretInput),
    SaveHyperdashKey,
    ToggleLiquidationOverlay(ChartId),
    ChartLiquidationLoaded(
        String,
        u64,
        RedactedHyperdashMarketMessageResult<LiquidationLevel>,
    ),
    RefreshLiquidations,
    LiquidationsDistributionLoaded(
        String,
        u64,
        RedactedHyperdashMarketMessageResult<LiquidationLevel>,
    ),
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
    ChartHeatmapLoaded(
        String,
        u64,
        RedactedHyperdashMarketMessageResult<LiquidationHeatmap>,
    ),
    RefreshHeatmap,
    // OpenRouter AI integration
    OpenRouterKeyInputChanged(SecretInput),
    SaveOpenRouterKey,
    OpenRouterKeyChecked(u64, Result<OpenRouterKeyStatus, String>),
    OpenRouterModelChanged(String),
}

#[cfg(test)]
mod tests {
    use super::{
        Message, RedactedAccountLabel, RedactedAccountMessageResult, RedactedAccountProfileId,
        RedactedAdvancedOrderHistoryId, RedactedClientOrderId,
        RedactedHyperdashMarketMessageResult, RedactedJournalMessageResult,
        RedactedLayoutMessageResult, RedactedOrderId, RedactedOrderInput,
        RedactedOrderMessageResult, RedactedOrderSymbol, RedactedOrderValue, RedactedPhoneInput,
        RedactedPnlCardMessageResult, RedactedPositioningMessageResult,
        RedactedPublicMarketMessageResult, RedactedTelegramChannelKey, RedactedWalletClusterId,
        RedactedWalletClusterName, RedactedWalletLabel, RedactedWalletLabelsMessageResult,
        SchwabAccountsMessageResult, SchwabTokenRefreshMessageResult, SecretInput,
        TelegramFastAuthMessageResult, TelegramFastAuthOutcome, XAccessTokenRefreshMessageResult,
        XAuthContextMessageResult, XFeedPageMessageResult, XListsMessageResult,
        XProfileImageMessageResult,
    };
    use crate::account_analytics::{PortfolioBucket, PortfolioHistory};
    use crate::api::{BookLevel, ExchangeSymbol, ExchangeSymbolsPayload, MarketType, OrderBook};
    use crate::chart_state::ChartSurfaceId;
    use crate::config::{ChartBackfillSource, MarketUniverseConfig, ReadDataProvider};
    use crate::hyperdash_api::{
        HeatmapRect, LiquidationEntry, LiquidationHeatmap, LiquidationLevel, PerpDeltaEntry,
        PerpDeltas, TickerPositionEntry, TickerPositions,
    };
    use crate::order_execution::{
        OneShotPlacementContext, OrderLeverageSubmissionSnapshot, PendingLeverageUpdateContext,
        QuickOrderForm, QuickOrderQuantityProvenance, QuickOrderRecovery,
    };
    use crate::read_data_provider::{
        AccountDataRequestContext, MarketDataSourceContext, ReadDataRequestContext,
    };
    use crate::timeframe::Timeframe;
    use crate::ws::{
        HydromancerWsMessage, TrackedTradeEvent, WsUserData, WsUserDataStreamParams,
        WsUserDataStreamPurpose,
    };
    use crate::x_feed::{XAuthenticatedUser, XListOwnerKind, XListSummary, XListsFetchOutcome};

    #[test]
    fn secret_input_debug_redacts_value() {
        let rendered = format!("{:?}", SecretInput::from("super-secret"));

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("super-secret"));
    }

    #[test]
    fn order_input_debug_redacts_value() {
        let input = RedactedOrderInput::from("order-input-secret");
        let rendered = format!("{input:?}");

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("order-input-secret"));
        assert_eq!(input.into_string(), "order-input-secret");
    }

    #[test]
    fn wallet_label_debug_redacts_and_preserves_exact_value() {
        const LABEL: &str = "private-wallet-label-input-sentinel";
        let label = RedactedWalletLabel::from(LABEL);

        let rendered = format!("{label:?}");

        assert!(rendered.contains("<redacted>"), "{rendered}");
        assert!(!rendered.contains(LABEL), "{rendered}");
        assert_eq!(label.into_string(), LABEL);
    }

    #[test]
    fn account_and_cluster_identity_wrappers_preserve_exact_values() {
        const ACCOUNT_LABEL: &str = "private-account-label-sentinel";
        const PROFILE_ID: &str = "private-account-profile-id-sentinel";
        const CLUSTER_ID: &str = "private-wallet-cluster-id-sentinel";
        const CLUSTER_NAME: &str = "private-wallet-cluster-name-sentinel";
        let account_label = RedactedAccountLabel::from(ACCOUNT_LABEL);
        let profile_id = RedactedAccountProfileId::from(PROFILE_ID);
        let cluster_id = RedactedWalletClusterId::from(CLUSTER_ID);
        let cluster_name = RedactedWalletClusterName::from(CLUSTER_NAME);

        for (rendered, sensitive) in [
            (format!("{account_label:?}"), ACCOUNT_LABEL),
            (format!("{profile_id:?}"), PROFILE_ID),
            (format!("{cluster_id:?}"), CLUSTER_ID),
            (format!("{cluster_name:?}"), CLUSTER_NAME),
        ] {
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(sensitive), "{rendered}");
        }
        assert_eq!(account_label.into_string(), ACCOUNT_LABEL);
        assert_eq!(profile_id.into_string(), PROFILE_ID);
        assert_eq!(cluster_id.into_string(), CLUSTER_ID);
        assert_eq!(cluster_name.into_string(), CLUSTER_NAME);
    }

    #[test]
    fn order_value_wrapper_preserves_exact_float_bits_and_preset() {
        const F64_BITS: u64 = 0x7ff8_1234_5678_9abc;
        const F32_BITS: u32 = 0x7fc1_2345;
        let f64_value = f64::from_bits(F64_BITS);
        let f32_value = f32::from_bits(F32_BITS);
        let preset = crate::config::OrderPreset {
            label: "exact-preset-label".to_string(),
            size: 12_345.678_901,
            price_offset_pct: Some(4.625),
        };
        let f64_value = RedactedOrderValue::from(f64_value);
        let f32_value = RedactedOrderValue::from(f32_value);
        let negative_zero = RedactedOrderValue::from(-0.0_f64);
        let wrapped_preset = RedactedOrderValue::from(preset.clone());

        for rendered in [
            format!("{f64_value:?}"),
            format!("{f32_value:?}"),
            format!("{wrapped_preset:?}"),
        ] {
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains("exact-preset-label"), "{rendered}");
            assert!(!rendered.contains("12345.678901"), "{rendered}");
        }

        assert_eq!(f64_value.into_inner().to_bits(), F64_BITS);
        assert_eq!(f32_value.into_inner().to_bits(), F32_BITS);
        assert_eq!(negative_zero.into_inner().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(wrapped_preset.into_inner(), preset);
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
            Message::OrderLeverageInputChanged("order-input-secret".into()),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"));
            assert!(!rendered.contains("order-input-secret"));
        }
    }

    #[test]
    fn remaining_mutation_result_message_debug_redacts_error_context() {
        const ERROR: &str = "remaining-mutation-external-error-sentinel";
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const CLOID: &str = "0x1234567890abcdef1234567890abcdef";
        let context = OneShotPlacementContext {
            account_address: ADDRESS.to_string(),
            cloid: CLOID.to_string(),
            surface: crate::order_execution::OrderSurface::Ticket,
            symbol_key: "HYPE".to_string(),
            order_kind: crate::signing::ExchangeOrderKind::Limit,
        };
        let messages = vec![
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
                result: Err(ERROR.to_string()).into(),
            },
            Message::WalletClusterOrderResult {
                execution_id: 1,
                member_key: Some("member-key-sentinel".to_string()).into(),
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::WalletClusterOrderStatusLoaded {
                execution_id: 1,
                member_key: Some("member-key-sentinel".to_string()).into(),
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::OrderResult {
                pending_indicator_id: Some(2),
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::CancelResult {
                request_id: 3,
                account_address: ADDRESS.into(),
                pending_indicator_id: Some(4),
                result: Err(ERROR.to_string()).into(),
            },
            Message::CancelOrderStatusLoaded {
                request_id: 3,
                account_address: ADDRESS.into(),
                oid: 9_876_543_210_123_457_u64.into(),
                symbol: "HYPE".into(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::ClosePositionResult {
                pending_indicator_id: Some(5),
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::NukeResult {
                execution_id: 6,
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::NukePlacementStatusLoaded {
                execution_id: 6,
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::OneShotPlacementStatusLoaded {
                request_id: 7,
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::QuickOrderResult {
                pending_indicator_id: Some(8),
                context: context.clone(),
                recovery: None,
                result: Err(ERROR.to_string()).into(),
            },
            Message::HudOrderResult {
                pending_indicator_id: Some(9),
                inflight_id: Some(10),
                context: context.clone(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::MoveOrderModifyResult {
                request_id: 11,
                account_address: ADDRESS.into(),
                coin: "HYPE".into(),
                oid: 9_876_543_210_123_457_u64.into(),
                pending_indicator_id: Some(12),
                result: Err(ERROR.to_string()).into(),
            },
            Message::MoveOrderStatusLoaded {
                request_id: 11,
                account_address: ADDRESS.into(),
                coin: "HYPE".into(),
                oid: 9_876_543_210_123_457_u64.into(),
                result: Err(ERROR.to_string()).into(),
            },
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(ERROR), "error leaked in {rendered}");
        }
    }

    #[test]
    fn remaining_order_symbol_and_history_message_debug_redacts_exact_values() {
        const SYMBOL: &str = "remaining-order-symbol-sentinel";
        const HISTORY_ID: &str = "history-entry-account-identity-sentinel";
        let messages = vec![
            Message::PrefillOutcomeSell(SYMBOL.into()),
            Message::WalletClusterClosePosition {
                symbol: SYMBOL.into(),
                side: crate::wallet_cluster_state::WalletClusterCloseSide::Long,
                fraction: 0.5.into(),
                use_market: true,
            },
            Message::CancelOrder {
                coin: SYMBOL.into(),
                oid: 9_876_543_210_123_457_u64.into(),
            },
            Message::CancelOrderStatusLoaded {
                request_id: 1,
                account_address: "synthetic-account".into(),
                oid: 9_876_543_210_123_457_u64.into(),
                symbol: SYMBOL.into(),
                result: Err("synthetic status error".to_string()).into(),
            },
            Message::ToggleCloseMenu(SYMBOL.into()),
            Message::ToggleHiddenPosition(SYMBOL.into()),
            Message::ClosePosition {
                coin: SYMBOL.into(),
                fraction: 0.25.into(),
                use_market: false,
            },
            Message::OpenAdvancedOrderHistory(HISTORY_ID.into()),
            Message::MoveOrderDragStarted {
                coin: SYMBOL.into(),
                oid: 9_876_543_210_123_457_u64.into(),
            },
            Message::MoveOrder {
                coin: SYMBOL.into(),
                oid: 9_876_543_210_123_457_u64.into(),
                new_price: 12_345.678_901.into(),
            },
            Message::MoveOrderModifyResult {
                request_id: 2,
                account_address: "synthetic-account".into(),
                coin: SYMBOL.into(),
                oid: 9_876_543_210_123_457_u64.into(),
                pending_indicator_id: Some(3),
                result: Err("synthetic modify error".to_string()).into(),
            },
            Message::MoveOrderStatusLoaded {
                request_id: 2,
                account_address: "synthetic-account".into(),
                coin: SYMBOL.into(),
                oid: 9_876_543_210_123_457_u64.into(),
                result: Err("synthetic status error".to_string()).into(),
            },
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(SYMBOL), "symbol leaked in {rendered}");
            assert!(
                !rendered.contains(HISTORY_ID),
                "history identity leaked in {rendered}"
            );
        }
    }

    #[test]
    fn remaining_financial_message_debug_redacts_exact_values() {
        const INPUT: &str = "remaining-financial-input-sentinel";
        const PRESET_LABEL: &str = "remaining-financial-preset-label-sentinel";
        const FINANCIAL_F64: f64 = 91_827.364_512_739;
        const FINANCIAL_F32: f32 = 73.125;
        const PRESET_OFFSET: f64 = 18.375;
        let financial_f64 = format!("{FINANCIAL_F64:?}");
        let financial_f32 = format!("{FINANCIAL_F32:?}");
        let preset_offset = format!("{PRESET_OFFSET:?}");
        let messages = vec![
            Message::OrderBookPriceSelected {
                id: 7,
                price: INPUT.into(),
            },
            Message::OrderPercentageChanged(FINANCIAL_F32.into()),
            Message::EditPresetStart(crate::signing::OrderKind::Limit, 3, INPUT.into()),
            Message::EditPresetChanged(INPUT.into()),
            Message::ExecutePreset(
                crate::signing::OrderKind::Limit,
                crate::config::OrderPreset {
                    label: PRESET_LABEL.to_string(),
                    size: FINANCIAL_F64,
                    price_offset_pct: Some(PRESET_OFFSET),
                }
                .into(),
                true,
            ),
            Message::MarketSlippageInputChanged(INPUT.into()),
            Message::WalletClusterClosePosition {
                symbol: "HYPE".into(),
                side: crate::wallet_cluster_state::WalletClusterCloseSide::Long,
                fraction: FINANCIAL_F64.into(),
                use_market: true,
            },
            Message::ClosePosition {
                coin: "HYPE".into(),
                fraction: FINANCIAL_F64.into(),
                use_market: false,
            },
            Message::OpenQuickOrder(
                7,
                ChartSurfaceId::Docked(7),
                FINANCIAL_F64.into(),
                11.0,
                12.0,
                13.0,
                14.0,
            ),
            Message::QuickOrderPercentageChanged(7, FINANCIAL_F32.into()),
            Message::MoveOrder {
                coin: "HYPE".into(),
                oid: 9_876_543_210_123_457_u64.into(),
                new_price: FINANCIAL_F64.into(),
            },
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            for sensitive in [
                INPUT,
                PRESET_LABEL,
                financial_f64.as_str(),
                financial_f32.as_str(),
                preset_offset.as_str(),
            ] {
                assert!(
                    !rendered.contains(sensitive),
                    "financial value {sensitive} leaked in {rendered}"
                );
            }
        }
    }

    #[test]
    fn advanced_order_message_debug_redacts_symbol_and_error_context() {
        const SYMBOL: &str = "advanced-order-symbol-sentinel";
        const ERROR: &str = "advanced-order-external-error-sentinel";
        const OID: u64 = 9_876_543_210_123_457;
        const CLOID: &str = "0x1234567890abcdef1234567890abcdef";
        let source_context = MarketDataSourceContext {
            provider: ReadDataProvider::Hyperliquid,
            read_data_provider_generation: 7,
            hydromancer_key_generation: None,
        };
        let book = OrderBook {
            bids: vec![BookLevel {
                px: 98_765.432_123,
                sz: 12.345_678_912,
            }],
            asks: vec![BookLevel {
                px: 98_766.543_234,
                sz: 23.456_789_123,
            }],
        };
        let messages = vec![
            Message::TwapBookUpdate {
                twap_id: 1,
                coin: SYMBOL.into(),
                sigfigs: (Some(5), None),
                source_context,
                book: book.clone(),
            },
            Message::TwapBookLagged {
                twap_id: 1,
                coin: SYMBOL.into(),
                sigfigs: (Some(5), None),
                source_context,
                skipped: 3,
            },
            Message::TwapSliceResult {
                twap_id: 1,
                slice_index: 2,
                retry_count: 0,
                result: Err(ERROR.to_string()).into(),
            },
            Message::TwapUnexpectedCancelResult {
                twap_id: 1,
                oid: Some(OID.into()),
                cloid: Some(CLOID.into()),
                attempt: 2,
                result: Err(ERROR.to_string()).into(),
            },
            Message::TwapOrderStatusLoaded {
                twap_id: 1,
                cloid: CLOID.into(),
                attempt: 2,
                result: Err(ERROR.to_string()).into(),
            },
            Message::ChaseInitialBookLoaded {
                chase_id: 2,
                result: Err(ERROR.to_string()).into(),
            },
            Message::ChaseBookUpdate {
                chase_id: 2,
                coin: SYMBOL.into(),
                sigfigs: (Some(5), None),
                source_context,
                book,
            },
            Message::ChaseBookLagged {
                chase_id: 2,
                coin: SYMBOL.into(),
                sigfigs: (Some(5), None),
                source_context,
                skipped: 4,
            },
            Message::ChasePlaceResult {
                chase_id: 2,
                place_attempt: 3,
                result: Err(ERROR.to_string()).into(),
            },
            Message::ChaseModifyResult {
                chase_id: 2,
                oid: OID.into(),
                reprice_count: 4,
                result: Err(ERROR.to_string()).into(),
            },
            Message::ChaseCancelResult {
                chase_id: 2,
                oid: OID.into(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::ChaseOrderStatusLoaded {
                chase_id: 2,
                cloid: CLOID.into(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::ChaseOrderOidStatusLoaded {
                chase_id: 2,
                oid: OID.into(),
                result: Err(ERROR.to_string()).into(),
            },
            Message::ChaseRestingOrder {
                coin: SYMBOL.into(),
                oid: OID.into(),
            },
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(SYMBOL), "symbol leaked in {rendered}");
            assert!(!rendered.contains(ERROR), "error leaked in {rendered}");
        }
    }

    #[test]
    fn advanced_order_message_wrappers_preserve_exact_values_and_result_shape() {
        const SYMBOL: &str = "advanced-order-symbol-sentinel";
        const HISTORY_ID: &str = "advanced-order-history-identity-sentinel";
        const ERROR: &str = "advanced-order-external-error-sentinel";
        const BID_PRICE: f64 = 98_765.432_123;
        let symbol = RedactedOrderSymbol::from(SYMBOL);
        let history_id = RedactedAdvancedOrderHistoryId::from(HISTORY_ID);
        let error: RedactedOrderMessageResult<OrderBook> = Err(ERROR.to_string()).into();
        let success: RedactedOrderMessageResult<OrderBook> = Ok(OrderBook {
            bids: vec![BookLevel {
                px: BID_PRICE,
                sz: 12.345_678_912,
            }],
            asks: Vec::new(),
        })
        .into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");
        let history_debug = format!("{history_id:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(history_debug.contains("<redacted>"), "{history_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(!history_debug.contains(HISTORY_ID), "{history_debug}");
        assert!(
            !success_debug.contains(&BID_PRICE.to_string()),
            "{success_debug}"
        );
        assert_eq!(symbol.into_string(), SYMBOL);
        assert_eq!(history_id.into_string(), HISTORY_ID);
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored_book = success.into_result().expect("synthetic book");
        assert_eq!(restored_book.bids[0].px, BID_PRICE);
    }

    #[test]
    fn account_message_result_wrapper_preserves_exact_payloads() {
        const ERROR: &str = "account-result-error-sentinel";
        const FINANCIAL_BITS: u64 = 0x7ff8_0000_0000_0042;
        let financial_value = f64::from_bits(FINANCIAL_BITS);
        let mut history = PortfolioHistory::default();
        history.buckets.insert(
            "day".to_string(),
            PortfolioBucket {
                account_value_history: vec![(123, financial_value)],
                ..PortfolioBucket::default()
            },
        );
        let error: RedactedAccountMessageResult<PortfolioHistory> = Err(ERROR.to_string()).into();
        let success: RedactedAccountMessageResult<PortfolioHistory> = Ok(history).into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(!success_debug.contains("day"), "{success_debug}");
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored = success.into_result().expect("synthetic portfolio history");
        assert_eq!(
            restored.buckets["day"].account_value_history[0].1.to_bits(),
            FINANCIAL_BITS
        );
    }

    #[test]
    fn positioning_message_result_wrapper_preserves_exact_payloads() {
        const ERROR: &str = "positioning-result-error-sentinel";
        const CURRENT_BITS: u64 = 0x7ff8_0000_0000_0053;
        const DELTA_BITS: u64 = 0x8000_0000_0000_0000;
        let error: RedactedPositioningMessageResult<PerpDeltas> = Err(ERROR.to_string()).into();
        let success: RedactedPositioningMessageResult<PerpDeltas> = Ok(PerpDeltas {
            market: "HYPE".to_string(),
            timeframe: "15m".to_string(),
            deltas: vec![PerpDeltaEntry {
                address: "synthetic-wallet-identity".to_string(),
                current: f64::from_bits(CURRENT_BITS),
                delta: f64::from_bits(DELTA_BITS),
            }],
        })
        .into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(
            !success_debug.contains("synthetic-wallet-identity"),
            "{success_debug}"
        );
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored = success.into_result().expect("synthetic positioning data");
        assert_eq!(restored.deltas[0].current.to_bits(), CURRENT_BITS);
        assert_eq!(restored.deltas[0].delta.to_bits(), DELTA_BITS);
    }

    #[test]
    fn hyperdash_market_message_result_wrapper_preserves_exact_payloads() {
        const ERROR: &str = "hyperdash-market-result-error-sentinel";
        const PRICE_BITS: u64 = 0x7ff8_0000_0000_0054;
        const AMOUNT_BITS: u64 = 0x8000_0000_0000_0000;
        let error: RedactedHyperdashMarketMessageResult<LiquidationHeatmap> =
            Err(ERROR.to_string()).into();
        let success: RedactedHyperdashMarketMessageResult<LiquidationHeatmap> =
            Ok(LiquidationHeatmap {
                rects: vec![HeatmapRect {
                    timestamp_ms: 1_778_357_590_000,
                    duration_ms: 3_600_000,
                    price_lo: f64::from_bits(PRICE_BITS),
                    price_hi: 2.0,
                    amount_coins: f64::from_bits(AMOUNT_BITS),
                    amount_usd: 3.0,
                }],
                max_abs_usd: 3.0,
            })
            .into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(!success_debug.contains("1778357590000"), "{success_debug}");
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored = success.into_result().expect("synthetic heatmap");
        assert_eq!(restored.rects[0].price_lo.to_bits(), PRICE_BITS);
        assert_eq!(restored.rects[0].amount_coins.to_bits(), AMOUNT_BITS);
    }

    #[test]
    fn public_market_message_result_wrapper_preserves_exact_payloads() {
        const ERROR: &str = "public-market-result-error-sentinel";
        const DAY_BITS: u64 = 0x7ff8_0000_0000_0055;
        const WEEK_BITS: u64 = 0x8000_0000_0000_0000;
        let error: RedactedPublicMarketMessageResult<
            std::collections::HashMap<String, (f64, f64, f64)>,
        > = Err(ERROR.to_string()).into();
        let success: RedactedPublicMarketMessageResult<
            std::collections::HashMap<String, (f64, f64, f64)>,
        > = Ok(std::collections::HashMap::from([(
            "payload-only-symbol".to_string(),
            (f64::from_bits(DAY_BITS), f64::from_bits(WEEK_BITS), 3.0),
        )]))
        .into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(
            !success_debug.contains("payload-only-symbol"),
            "{success_debug}"
        );
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored = success.into_result().expect("synthetic market history");
        assert_eq!(restored["payload-only-symbol"].0.to_bits(), DAY_BITS);
        assert_eq!(restored["payload-only-symbol"].1.to_bits(), WEEK_BITS);
    }

    #[test]
    fn journal_message_result_wrapper_preserves_exact_candles_and_error() {
        const ERROR: &str = "journal-result-error-sentinel";
        const OPEN_TIME: u64 = 9_123_456_789;
        const FINANCIAL_BITS: u64 = 0x7ff8_0000_0000_0042;
        let financial_value = f64::from_bits(FINANCIAL_BITS);
        let candle = crate::api::Candle::test_ohlcv(
            OPEN_TIME,
            OPEN_TIME + 59_999,
            [financial_value, 20.0, 10.0, 15.0],
            42.0,
        );
        let error: RedactedJournalMessageResult<Vec<crate::api::Candle>> =
            Err(ERROR.to_string()).into();
        let success: RedactedJournalMessageResult<Vec<crate::api::Candle>> =
            Ok(vec![candle]).into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(
            !success_debug.contains(&OPEN_TIME.to_string()),
            "{success_debug}"
        );
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored = success.into_result().expect("synthetic candles");
        assert_eq!(restored[0].open_time, OPEN_TIME);
        assert_eq!(restored[0].open.to_bits(), FINANCIAL_BITS);
    }

    #[test]
    fn pnl_card_message_result_wrapper_preserves_exact_path_and_error() {
        const ERROR: &str = "pnl-card-result-error-sentinel";
        const PATH_COMPONENT: &str = "pnl-card-result-path-sentinel";
        let path = std::path::PathBuf::from(format!("{PATH_COMPONENT}/card.png"));
        let error: RedactedPnlCardMessageResult<Option<std::path::PathBuf>> =
            Err(ERROR.to_string()).into();
        let success: RedactedPnlCardMessageResult<Option<std::path::PathBuf>> =
            Ok(Some(path.clone())).into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(!success_debug.contains(PATH_COMPONENT), "{success_debug}");
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        assert_eq!(
            success.into_result().expect("synthetic success"),
            Some(path)
        );
    }

    #[test]
    fn layout_message_result_wrapper_preserves_exact_layout_and_error() {
        const NAME: &str = "layout-result-name-sentinel";
        const SYMBOL: &str = "layout-result-symbol-sentinel";
        const ERROR: &str = "layout-result-error-sentinel";
        let layout: crate::config::SavedLayout = serde_json::from_value(serde_json::json!({
            "name": NAME,
            "active_symbol": SYMBOL,
            "market_slippage_pct": 6.54321
        }))
        .expect("synthetic saved layout");
        let wire = serde_json::to_value(&layout).expect("serialize synthetic layout");
        let error: RedactedLayoutMessageResult<crate::config::SavedLayout> =
            Err(ERROR.to_string()).into();
        let success: RedactedLayoutMessageResult<crate::config::SavedLayout> = Ok(layout).into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(!success_debug.contains(NAME), "{success_debug}");
        assert!(!success_debug.contains(SYMBOL), "{success_debug}");
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored = success.into_result().expect("synthetic success");
        assert_eq!(
            serde_json::to_value(restored).expect("serialize restored layout"),
            wire
        );
    }

    #[test]
    fn wallet_labels_message_result_wrapper_preserves_exact_export_and_error() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const LABEL: &str = "wallet-label-result-label-sentinel";
        const ERROR: &str = "wallet-label-result-error-sentinel";
        let export = crate::config::WalletLabelsExport {
            schema: crate::config::WALLET_LABELS_EXPORT_SCHEMA.to_string(),
            exported_at_ms: 9_123_456_789,
            labels: vec![crate::config::AddressBookEntryConfig {
                address: ADDRESS.to_string(),
                label: LABEL.to_string(),
                color: Some("#a1b2c3".to_string()),
                tags: vec!["wallet-label-result-tag-sentinel".to_string()],
            }],
        };
        let wire = serde_json::to_value(&export).expect("serialize synthetic wallet labels");
        let error: RedactedWalletLabelsMessageResult<crate::config::WalletLabelsExport> =
            Err(ERROR.to_string()).into();
        let success: RedactedWalletLabelsMessageResult<crate::config::WalletLabelsExport> =
            Ok(export).into();

        let error_debug = format!("{error:?}");
        let success_debug = format!("{success:?}");

        assert!(error_debug.contains("Err(<redacted>)"), "{error_debug}");
        assert!(success_debug.contains("Ok(<redacted>)"), "{success_debug}");
        assert!(!error_debug.contains(ERROR), "{error_debug}");
        assert!(!success_debug.contains(ADDRESS), "{success_debug}");
        assert!(!success_debug.contains(LABEL), "{success_debug}");
        assert_eq!(error.into_result().expect_err("synthetic error"), ERROR);
        let restored = success.into_result().expect("synthetic wallet labels");
        assert_eq!(
            serde_json::to_value(restored).expect("serialize restored wallet labels"),
            wire
        );
    }

    #[test]
    fn leverage_message_debug_redacts_mutation_parameters() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const SYMBOL: &str = "leverage-symbol-sentinel";
        const DISPLAY: &str = "leverage-display-sentinel";
        const DEX: &str = "leverage-dex-sentinel";
        const INPUT: &str = "leverage-input-sentinel";
        const ASSET: u32 = 110_003;
        const LEVERAGE: u32 = 73;
        let asset = ASSET.to_string();
        let leverage = LEVERAGE.to_string();

        let messages = [
            Message::SubmitOrderLeverage(OrderLeverageSubmissionSnapshot {
                symbol_key: SYMBOL.to_string(),
                leverage_input: INPUT.to_string(),
                is_cross: true,
            }),
            Message::OrderLeverageResult {
                context: PendingLeverageUpdateContext {
                    address: ADDRESS.to_string(),
                    symbol_key: SYMBOL.to_string(),
                    display: DISPLAY.to_string(),
                    asset: ASSET,
                    dex: Some(DEX.to_string()),
                    is_cross: false,
                    leverage: LEVERAGE,
                },
                result: Err("leverage failed".to_string()).into(),
            },
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            for sensitive in [
                ADDRESS,
                SYMBOL,
                DISPLAY,
                DEX,
                INPUT,
                asset.as_str(),
                leverage.as_str(),
            ] {
                assert!(
                    !rendered.contains(sensitive),
                    "{sensitive} leaked: {rendered}"
                );
            }
        }
    }

    #[test]
    fn order_identifier_message_debug_redacts_oid_and_cloid_fields() {
        const OID: u64 = 9_876_543_210_123_457;
        const CLOID: &str = "0x1234567890abcdef1234567890abcdef";

        let messages = vec![
            Message::CancelOrder {
                coin: "HYPE".into(),
                oid: OID.into(),
            },
            Message::CancelOrderStatusLoaded {
                request_id: 1,
                account_address: "0x0000000000000000000000000000000000000001".into(),
                oid: OID.into(),
                symbol: "HYPE".into(),
                result: Err("status failed".to_string()).into(),
            },
            Message::TwapUnexpectedCancelResult {
                twap_id: 1,
                oid: Some(OID.into()),
                cloid: Some(CLOID.into()),
                attempt: 0,
                result: Err("cancel failed".to_string()).into(),
            },
            Message::TwapUnexpectedCancelRetryDue {
                twap_id: 1,
                oid: Some(OID.into()),
                cloid: Some(CLOID.into()),
                attempt: 1,
            },
            Message::TwapOrderStatusLoaded {
                twap_id: 1,
                cloid: CLOID.into(),
                attempt: 0,
                result: Err("status failed".to_string()).into(),
            },
            Message::ChaseModifyResult {
                chase_id: 1,
                oid: OID.into(),
                reprice_count: 1,
                result: Err("modify failed".to_string()).into(),
            },
            Message::ChaseCancelResult {
                chase_id: 1,
                oid: OID.into(),
                result: Err("cancel failed".to_string()).into(),
            },
            Message::ChaseOrderStatusLoaded {
                chase_id: 1,
                cloid: CLOID.into(),
                result: Err("status failed".to_string()).into(),
            },
            Message::ChaseOrderOidStatusLoaded {
                chase_id: 1,
                oid: OID.into(),
                result: Err("status failed".to_string()).into(),
            },
            Message::ChaseRestingOrder {
                coin: "HYPE".into(),
                oid: OID.into(),
            },
            Message::MoveOrderDragStarted {
                coin: "HYPE".into(),
                oid: OID.into(),
            },
            Message::MoveOrder {
                coin: "HYPE".into(),
                oid: OID.into(),
                new_price: 100.0.into(),
            },
            Message::MoveOrderModifyResult {
                request_id: 2,
                account_address: "0x0000000000000000000000000000000000000001".into(),
                coin: "HYPE".into(),
                oid: OID.into(),
                pending_indicator_id: None,
                result: Err("modify failed".to_string()).into(),
            },
            Message::MoveOrderStatusLoaded {
                request_id: 2,
                account_address: "0x0000000000000000000000000000000000000001".into(),
                coin: "HYPE".into(),
                oid: OID.into(),
                result: Err("status failed".to_string()).into(),
            },
            Message::ChartHoverStateChanged(
                1,
                ChartSurfaceId::Docked(1),
                Some(OID.into()),
                true,
                None,
            ),
        ];
        let oid = OID.to_string();

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(&oid), "message leaked OID: {rendered}");
            assert!(
                !rendered.contains(CLOID),
                "message leaked CLOID: {rendered}"
            );
        }
    }

    #[test]
    fn order_identifier_message_wrappers_preserve_exact_values() {
        const OID: u64 = 9_876_543_210_123_457;
        const CLOID: &str = "0x1234567890abcdef1234567890abcdef";

        assert_eq!(RedactedOrderId::from(OID).into_u64(), OID);
        assert_eq!(RedactedClientOrderId::from(CLOID).into_string(), CLOID);
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
            Message::XFeedAccessTokenChanged("sentinel-secret".into()),
            Message::XFeedOAuthClientIdChanged("sentinel-secret".into()),
            Message::XFeedRefreshTokenChanged("sentinel-secret".into()),
            Message::XAccessTokenRefreshed(
                1,
                XAccessTokenRefreshMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::XAccessTokenRefreshed(
                2,
                XAccessTokenRefreshMessageResult::new(Ok(crate::x_feed::XOAuthTokenRefresh {
                    access_token: "sentinel-secret".to_string().into(),
                    refresh_token: Some("sentinel-secret".to_string().into()),
                    expires_in_secs: Some(7_200),
                })),
            ),
            Message::XFeedAuthLoaded(
                1,
                XAuthContextMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::XFeedAuthLoaded(
                2,
                XAuthContextMessageResult::new(Ok((
                    XAuthenticatedUser {
                        id: "sentinel-secret".to_string(),
                        username: "alice".to_string(),
                        name: "sentinel-secret".to_string(),
                    },
                    XListsFetchOutcome {
                        lists: vec![XListSummary {
                            id: "10".to_string(),
                            name: "sentinel-secret".to_string(),
                            private: false,
                            owner: XListOwnerKind::Owned,
                        }],
                        unavailable_sources: vec![XListOwnerKind::Followed],
                    },
                ))),
            ),
            Message::XFeedListsLoaded(
                3,
                XListsMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::XFeedLoaded(
                crate::x_feed::XFeedSource::Following,
                4,
                XFeedPageMessageResult::new(Err(crate::x_feed::XFeedRequestError::plain(
                    "sentinel-secret",
                ))),
            ),
            Message::XProfileImageLoaded(
                5,
                XProfileImageMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::WalletKeyInputChanged("sentinel-secret".into()),
            Message::AddAccountKeyChanged("sentinel-secret".into()),
            Message::HydromancerKeyInputChanged("sentinel-secret".into()),
            Message::HyperdashKeyInputChanged("sentinel-secret".into()),
            Message::SchwabClientIdChanged("sentinel-secret".into()),
            Message::SchwabClientSecretChanged("sentinel-secret".into()),
            Message::SchwabAccessTokenChanged("sentinel-secret".into()),
            Message::SchwabRefreshTokenChanged("sentinel-secret".into()),
            Message::SchwabAccessTokenRefreshed(
                6,
                SchwabTokenRefreshMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::SchwabAccessTokenRefreshed(
                7,
                SchwabTokenRefreshMessageResult::new(Ok(crate::schwab::SchwabOAuthTokenRefresh {
                    access_token: "sentinel-secret".to_string().into(),
                    refresh_token: Some("sentinel-secret".to_string().into()),
                    expires_in_secs: Some(1_800),
                })),
            ),
            Message::SchwabAccountsLoaded(
                8,
                SchwabAccountsMessageResult::new(Err("sentinel-secret".to_string())),
            ),
            Message::SchwabAccountsLoaded(
                9,
                SchwabAccountsMessageResult::new(Ok(crate::schwab::SchwabAccountsSnapshot {
                    linked_accounts: vec![crate::schwab::SchwabLinkedAccount {
                        account_number: Some("sentinel-secret".to_string()),
                        hash_value: "sentinel-secret".to_string(),
                    }],
                    accounts: vec![crate::schwab::SchwabAccountSummary {
                        account_number: Some("sentinel-secret".to_string()),
                        account_hash: "sentinel-secret".to_string(),
                        account_type: Some("BROKERAGE".to_string()),
                        cash_balance: Some(1.0),
                        buying_power: Some(2.0),
                        liquidation_value: Some(3.0),
                        positions: vec![crate::schwab::SchwabPositionSummary {
                            symbol: "AAPL".to_string(),
                            quantity: 4.0,
                            market_value: Some(5.0),
                        }],
                    }],
                })),
            ),
            Message::SchwabAccountPickerSelected(Some("sentinel-secret".to_string()).into()),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(
                rendered.contains("<redacted>"),
                "missing redaction marker: {rendered}"
            );
            assert!(
                !rendered.contains("sentinel-secret"),
                "debug output leaked a secret: {rendered}"
            );
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
    fn symbols_loaded_message_debug_is_value_neutral() {
        const PAYLOAD_SYMBOL: &str = "private-symbol-payload-sentinel";
        const ERROR: &str = "private-symbol-result-error-sentinel";
        let payload = ExchangeSymbolsPayload {
            symbols: vec![ExchangeSymbol {
                key: PAYLOAD_SYMBOL.to_string(),
                ticker: PAYLOAD_SYMBOL.to_string(),
                category: "crypto".to_string(),
                display_name: Some(PAYLOAD_SYMBOL.to_string()),
                keywords: vec![PAYLOAD_SYMBOL.to_string()],
                asset_index: 0,
                collateral_token: Some(0),
                sz_decimals: 0,
                max_leverage: 1,
                only_isolated: false,
                market_type: MarketType::Perp,
                outcome: None,
            }],
            loaded_from_cache: false,
            perp_meta_failed: false,
            spot_meta_failed: false,
            outcome_meta_failed: false,
        };
        let messages = [
            Message::SymbolsLoaded(7, Ok(payload).into()),
            Message::SymbolsLoaded(8, Err(ERROR.to_string()).into()),
        ];

        for (message, request_id) in messages.into_iter().zip([7_u64, 8]) {
            let rendered = format!("{message:?}");

            assert!(rendered.contains("SymbolsLoaded"), "{rendered}");
            assert!(rendered.contains(&request_id.to_string()), "{rendered}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(PAYLOAD_SYMBOL), "{rendered}");
            assert!(!rendered.contains(ERROR), "{rendered}");
        }
    }

    #[test]
    fn pnl_card_message_debug_redacts_target_results_and_path() {
        const SYMBOL: &str = "private-pnl-message-symbol-sentinel";
        const ERROR: &str = "private-pnl-export-error-sentinel";
        const PATH_COMPONENT: &str = "private-pnl-save-path-sentinel";
        let messages = [
            Message::OpenPnlCard(crate::pnl_card::PnlCardTarget::Position(SYMBOL.to_string())),
            Message::PnlCardCopied(Err(ERROR.to_string()).into()),
            Message::PnlCardSaved(
                Ok(Some(std::path::PathBuf::from(format!(
                    "{PATH_COMPONENT}/pnl-card.png"
                ))))
                .into(),
            ),
            Message::PnlCardSaved(Err(ERROR.to_string()).into()),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(SYMBOL), "{rendered}");
            assert!(!rendered.contains(ERROR), "{rendered}");
            assert!(!rendered.contains(PATH_COMPONENT), "{rendered}");
        }
    }

    #[test]
    fn layout_message_debug_redacts_saved_order_config_and_external_errors() {
        const NAME: &str = "private-message-layout-name-sentinel";
        const SYMBOL: &str = "private-message-layout-symbol-sentinel";
        const PRESET_LABEL: &str = "private-message-preset-label-sentinel";
        const ERROR: &str = "private-layout-io-error-sentinel";
        let layout: crate::config::SavedLayout = serde_json::from_value(serde_json::json!({
            "name": NAME,
            "active_symbol": SYMBOL,
            "market_slippage_pct": 7.654321,
            "order_presets": {
                "market_usd": [{
                    "label": PRESET_LABEL,
                    "size": 98765.4321,
                    "price_offset_pct": 1.234567
                }]
            }
        }))
        .expect("synthetic saved layout");
        let messages = [
            Message::LoadLayout(layout.clone()),
            Message::ExportLayout(layout.clone()),
            Message::LayoutImported(Ok(layout).into()),
            Message::LayoutImported(Err(ERROR.to_string()).into()),
            Message::LayoutExported(Err(ERROR.to_string()).into()),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(NAME), "{rendered}");
            assert!(!rendered.contains(SYMBOL), "{rendered}");
            assert!(!rendered.contains(PRESET_LABEL), "{rendered}");
            assert!(!rendered.contains(ERROR), "{rendered}");
            assert!(!rendered.contains("98765.4321"), "{rendered}");
            assert!(!rendered.contains("7.654321"), "{rendered}");
        }
    }

    #[test]
    fn wallet_label_message_debug_redacts_import_payload_and_external_errors() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const LABEL: &str = "private-wallet-message-label-sentinel";
        const COLOR: &str = "#c3b2a1";
        const TAG: &str = "private-wallet-message-tag-sentinel";
        const ERROR: &str = "private-wallet-label-io-error-sentinel";
        const EXPORTED_AT_MS: u64 = 9_876_543_210;
        let export = crate::config::WalletLabelsExport {
            schema: crate::config::WALLET_LABELS_EXPORT_SCHEMA.to_string(),
            exported_at_ms: EXPORTED_AT_MS,
            labels: vec![crate::config::AddressBookEntryConfig {
                address: ADDRESS.to_string(),
                label: LABEL.to_string(),
                color: Some(COLOR.to_string()),
                tags: vec![TAG.to_string()],
            }],
        };
        let messages = [
            Message::WalletLabelsImported(Ok(export).into()),
            Message::WalletLabelsImported(Err(ERROR.to_string()).into()),
            Message::WalletLabelsExported(Err(ERROR.to_string()).into()),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            for sensitive in [ADDRESS, LABEL, COLOR, TAG, ERROR] {
                assert!(
                    !rendered.contains(sensitive),
                    "{sensitive} leaked in {rendered}"
                );
            }
            assert!(
                !rendered.contains(&EXPORTED_AT_MS.to_string()),
                "timestamp leaked in {rendered}"
            );
        }
    }

    #[test]
    fn account_and_cluster_identity_message_debug_is_value_neutral() {
        const ACCOUNT_LABEL: &str = "private-account-message-label-sentinel";
        const PROFILE_ID: &str = "private-cluster-message-profile-id-sentinel";
        const CLUSTER_ID: &str = "private-cluster-message-id-sentinel";
        const CLUSTER_NAME: &str = "private-cluster-message-name-sentinel";
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const WEIGHT: &str = "7.654321";
        const ERROR: &str = "private-cluster-member-error-sentinel";
        let read_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hyperliquid,
            read_data_provider_generation: 1,
            hydromancer_key_generation: 2,
        };
        let messages = vec![
            Message::AccountPickerLabelChanged(1, ACCOUNT_LABEL.into()),
            Message::AddAccountNameChanged(ACCOUNT_LABEL.into()),
            Message::WalletClusterNameInputChanged(CLUSTER_NAME.into()),
            Message::WalletClusterSelected(CLUSTER_ID.into()),
            Message::WalletClusterRenamed(CLUSTER_ID.into(), CLUSTER_NAME.into()),
            Message::WalletClusterDeleted(CLUSTER_ID.into()),
            Message::WalletClusterAddMember(PROFILE_ID.into()),
            Message::WalletClusterRemoveMember(
                CLUSTER_ID.into(),
                Some(PROFILE_ID.to_string()).into(),
            ),
            Message::WalletClusterMemberWeightChanged(
                CLUSTER_ID.into(),
                Some(PROFILE_ID.to_string()).into(),
                WEIGHT.into(),
            ),
            Message::WalletClusterMemberLoaded(
                CLUSTER_ID.into(),
                Some(PROFILE_ID.to_string()).into(),
                ADDRESS.into(),
                read_context,
                Err(ERROR.to_string()).into(),
            ),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            for sensitive in [
                ACCOUNT_LABEL,
                PROFILE_ID,
                CLUSTER_ID,
                CLUSTER_NAME,
                ADDRESS,
                WEIGHT,
                ERROR,
            ] {
                assert!(
                    !rendered.contains(sensitive),
                    "{sensitive} leaked in {rendered}"
                );
            }
        }
    }

    #[test]
    fn positioning_result_message_debug_is_value_neutral() {
        const ADDRESS: &str = "synthetic-positioning-wallet-identity";
        const DISPLAY_NAME: &str = "private-positioning-display-name-sentinel";
        const LABEL: &str = "private-positioning-label-sentinel";
        const TAG: &str = "private-positioning-tag-sentinel";
        const ERROR: &str = "private-positioning-result-error-sentinel";
        const POSITION_VALUE: f64 = 918_273.125;
        const DELTA_VALUE: f64 = -827_364.25;
        let positions = TickerPositions {
            coin: "HYPE".to_string(),
            positions: vec![TickerPositionEntry {
                address: ADDRESS.to_string(),
                display_name: Some(DISPLAY_NAME.to_string()),
                label: Some(LABEL.to_string()),
                tag: Some(TAG.to_string()),
                verified: Some(true),
                copy_score: Some(61.5),
                size: POSITION_VALUE,
                notional_size: POSITION_VALUE,
                entry_price: POSITION_VALUE,
                liquidation_price: Some(POSITION_VALUE),
                unrealized_pnl: POSITION_VALUE,
                funding_pnl: POSITION_VALUE,
                account_value: POSITION_VALUE,
            }],
            total_long_notional: 600.0,
            total_short_notional: 400.0,
            total_notional: 1000.0,
            long_count: 3,
            short_count: 2,
            total_count: 5,
            has_more: true,
            timestamp: "2026-05-18T11:52:39.585Z".to_string(),
        };
        let deltas = PerpDeltas {
            market: "HYPE".to_string(),
            timeframe: "15m".to_string(),
            deltas: vec![PerpDeltaEntry {
                address: ADDRESS.to_string(),
                current: POSITION_VALUE,
                delta: DELTA_VALUE,
            }],
        };
        let messages = vec![
            Message::PositioningInfoLoaded(
                "HYPE:all:notional:desc:-:-:100:0".to_string(),
                7,
                Ok(positions).into(),
            ),
            Message::PositioningInfoLoaded(
                "HYPE:all:notional:desc:-:-:100:0".to_string(),
                7,
                Err(ERROR.to_string()).into(),
            ),
            Message::PositioningInfoChangeLoaded(
                "change:HYPE:FIFTEEN_MINUTES".to_string(),
                7,
                Ok(deltas).into(),
            ),
            Message::PositioningInfoChangeLoaded(
                "change:HYPE:FIFTEEN_MINUTES".to_string(),
                7,
                Err(ERROR.to_string()).into(),
            ),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            for sensitive in [
                ADDRESS,
                DISPLAY_NAME,
                LABEL,
                TAG,
                ERROR,
                "918273.125",
                "-827364.25",
            ] {
                assert!(
                    !rendered.contains(sensitive),
                    "{sensitive} leaked in {rendered}"
                );
            }
        }
    }

    #[test]
    fn hyperdash_public_market_result_message_debug_is_value_neutral() {
        const PUBLIC_REQUEST_KEY: &str = "PUBLIC-HYPERDASH-REQUEST-CONTEXT";
        const ERROR: &str = "private-hyperdash-market-result-error-sentinel";
        const AMOUNT: f64 = 918_273.125;
        const PRICE: f64 = 827_364.25;
        const MAX_USD: f64 = 736_455.375;
        let level = LiquidationLevel {
            coin: "PUBLIC-PERP".to_string(),
            min: 0.0,
            max: PRICE,
            liquidations: vec![LiquidationEntry {
                amount: AMOUNT,
                price: PRICE,
            }],
        };
        let heatmap = LiquidationHeatmap {
            rects: vec![HeatmapRect {
                timestamp_ms: 1_778_357_590_000,
                duration_ms: 3_600_000,
                price_lo: 0.0,
                price_hi: PRICE,
                amount_coins: AMOUNT,
                amount_usd: MAX_USD,
            }],
            max_abs_usd: MAX_USD,
        };
        let messages = vec![
            Message::ChartLiquidationLoaded(
                PUBLIC_REQUEST_KEY.to_string(),
                7,
                Ok(level.clone()).into(),
            ),
            Message::ChartLiquidationLoaded(
                PUBLIC_REQUEST_KEY.to_string(),
                7,
                Err(ERROR.to_string()).into(),
            ),
            Message::LiquidationsDistributionLoaded(
                PUBLIC_REQUEST_KEY.to_string(),
                7,
                Ok(level).into(),
            ),
            Message::LiquidationsDistributionLoaded(
                PUBLIC_REQUEST_KEY.to_string(),
                7,
                Err(ERROR.to_string()).into(),
            ),
            Message::ChartHeatmapLoaded(PUBLIC_REQUEST_KEY.to_string(), 7, Ok(heatmap).into()),
            Message::ChartHeatmapLoaded(
                PUBLIC_REQUEST_KEY.to_string(),
                7,
                Err(ERROR.to_string()).into(),
            ),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains(PUBLIC_REQUEST_KEY), "{rendered}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            for sensitive in [ERROR, "918273.125", "827364.25", "736455.375"] {
                assert!(
                    !rendered.contains(sensitive),
                    "{sensitive} leaked in {rendered}"
                );
            }
        }
    }

    #[test]
    fn public_market_refresh_result_message_debug_is_value_neutral() {
        const REQUEST_SYMBOL: &str = "PUBLIC-REQUEST-SYMBOL";
        const PAYLOAD_SYMBOL: &str = "PAYLOAD-ONLY-SYMBOL";
        const ERROR: &str = "private-public-market-result-error-sentinel";
        const PARTIAL_ERROR: &str = "private-public-market-partial-error-sentinel";
        const DAY_VALUE: f64 = 918_273.125;
        const WEEK_VALUE: f64 = 827_364.25;
        const MONTH_VALUE: f64 = 736_455.375;
        let contexts_response = || crate::api::WatchlistContextsResponse {
            contexts: std::collections::HashMap::from([(
                PAYLOAD_SYMBOL.to_string(),
                crate::api::WatchlistContext {
                    funding: Some(DAY_VALUE),
                    prev_day_px: Some(WEEK_VALUE),
                    day_vlm: Some(MONTH_VALUE),
                },
            )]),
            partial_errors: vec![PARTIAL_ERROR.to_string()],
        };
        let request_symbols = || vec![REQUEST_SYMBOL.to_string()];
        let messages = vec![
            Message::LiveWatchlistContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Ok(contexts_response()).into(),
            ),
            Message::LiveWatchlistContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Err(ERROR.to_string()).into(),
            ),
            Message::LiveWatchlistHistoryLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Ok(std::collections::HashMap::from([(
                    PAYLOAD_SYMBOL.to_string(),
                    (DAY_VALUE, WEEK_VALUE, MONTH_VALUE),
                )]))
                .into(),
            ),
            Message::LiveWatchlistHistoryLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Err(ERROR.to_string()).into(),
            ),
            Message::TickerTapeContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Ok(contexts_response()).into(),
            ),
            Message::TickerTapeContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Err(ERROR.to_string()).into(),
            ),
            Message::ScreenerContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Ok(contexts_response()).into(),
            ),
            Message::ScreenerContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Err(ERROR.to_string()).into(),
            ),
            Message::SymbolSearchContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Ok(contexts_response()).into(),
            ),
            Message::SymbolSearchContextsLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Err(ERROR.to_string()).into(),
            ),
            Message::OutcomeVolumesLoaded(
                7,
                request_symbols(),
                Ok(std::collections::HashMap::from([(
                    PAYLOAD_SYMBOL.to_string(),
                    crate::api::OutcomeVolume24h {
                        contract: DAY_VALUE,
                        notional: WEEK_VALUE,
                    },
                )]))
                .into(),
            ),
            Message::OutcomeVolumesLoaded(7, request_symbols(), Err(ERROR.to_string()).into()),
            Message::ScreenerHistoryLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Ok(std::collections::HashMap::from([(
                    PAYLOAD_SYMBOL.to_string(),
                    (DAY_VALUE, WEEK_VALUE),
                )]))
                .into(),
            ),
            Message::ScreenerHistoryLoaded(
                7,
                request_symbols(),
                1_778_357_590_000,
                Err(ERROR.to_string()).into(),
            ),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains(REQUEST_SYMBOL), "{rendered}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            for hidden in [
                PAYLOAD_SYMBOL,
                ERROR,
                PARTIAL_ERROR,
                "918273.125",
                "827364.25",
                "736455.375",
            ] {
                assert!(!rendered.contains(hidden), "{hidden} leaked in {rendered}");
            }
        }
    }

    #[test]
    fn boxed_account_result_message_debug_redacts_payloads() {
        const ERROR_SENTINEL: &str = "raw-provider-account-error-sentinel";
        const FINANCIAL_SENTINEL: f64 = 918_273_645.125;
        const FINANCIAL_SENTINEL_TEXT: &str = "918273645.125";

        let read_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hyperliquid,
            read_data_provider_generation: 1,
            hydromancer_key_generation: 2,
        };
        let account_context = AccountDataRequestContext::connected_snapshot(read_context, 3, 4);
        let mut portfolio_history = PortfolioHistory::default();
        portfolio_history.buckets.insert(
            "day".to_string(),
            PortfolioBucket {
                account_value_history: vec![(100, FINANCIAL_SENTINEL)],
                pnl_history: vec![(100, -FINANCIAL_SENTINEL)],
                vlm: Some(FINANCIAL_SENTINEL),
                ..PortfolioBucket::default()
            },
        );

        let messages = vec![
            Message::WalletClusterMemberLoaded(
                "cluster-1".into(),
                Some("profile-1".to_string()).into(),
                "0x1111111111111111111111111111111111111111".into(),
                read_context,
                Err(ERROR_SENTINEL.to_string()).into(),
            ),
            Message::WalletDetailsLoaded(
                iced::window::Id::unique(),
                "0x2222222222222222222222222222222222222222".into(),
                read_context,
                Err(ERROR_SENTINEL.to_string()).into(),
            ),
            Message::WalletTrackerLoaded(
                "0x3333333333333333333333333333333333333333".into(),
                read_context,
                Err(ERROR_SENTINEL.to_string()).into(),
            ),
            Message::WalletTrackerOrdersLoaded(
                "0x4444444444444444444444444444444444444444".into(),
                read_context,
                Err(ERROR_SENTINEL.to_string()).into(),
            ),
            Message::PortfolioLoaded(
                "0x5555555555555555555555555555555555555555".into(),
                5,
                Ok(portfolio_history).into(),
            ),
            Message::IncomeLoaded(
                "0x6666666666666666666666666666666666666666".into(),
                6,
                Err(ERROR_SENTINEL.to_string()).into(),
            ),
            Message::AccountDataLoaded(
                "0x7777777777777777777777777777777777777777".into(),
                account_context,
                Err(ERROR_SENTINEL.to_string()).into(),
            ),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(ERROR_SENTINEL), "{rendered}");
            assert!(!rendered.contains(FINANCIAL_SENTINEL_TEXT), "{rendered}");
        }
    }

    #[test]
    fn address_bearing_message_debug_redacts_values() {
        const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
        const ACCOUNT_KEY: &str = "account-key-sentinel";
        const JOURNAL_ERROR: &str = "journal-message-error-sentinel";
        const WALLET_LABEL: &str = "private-wallet-message-label-sentinel";

        let read_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hyperliquid,
            read_data_provider_generation: 1,
            hydromancer_key_generation: 2,
        };
        let account_context = AccountDataRequestContext::connected_snapshot(read_context, 3, 4);
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
                Err("details failed".to_string()).into(),
            ),
            Message::WalletDetailsWsUpdate(
                WsUserDataStreamParams::without_mids(Some(ADDRESS.to_string()), Vec::new())
                    .with_purpose(WsUserDataStreamPurpose::WalletDetail)
                    .with_generation(1),
                Some(ADDRESS.into()),
                Box::new(WsUserData::Lagged { skipped: 1 }),
            ),
            Message::JournalFillsLoaded {
                request_id: 1,
                account_key: Some(ACCOUNT_KEY.to_string()).into(),
                address: ADDRESS.into(),
                result: Err(JOURNAL_ERROR.to_string()).into(),
            },
            Message::JournalSnapshotLoaded {
                account_key: Some(ACCOUNT_KEY.to_string()).into(),
                address: ADDRESS.into(),
                request: snapshot_request.into(),
                result: Err(JOURNAL_ERROR.to_string()).into(),
            },
            Message::WalletTrackerInputChanged(ADDRESS.into()),
            Message::WalletTrackerLabelInputChanged(WALLET_LABEL.into()),
            Message::WalletTrackerMute(ADDRESS.into()),
            Message::WalletTrackerUnmute(ADDRESS.into()),
            Message::WalletTrackerRemove(ADDRESS.into()),
            Message::WalletTrackerLabelChanged(ADDRESS.into(), WALLET_LABEL.into()),
            Message::WalletTrackerRefreshOne(ADDRESS.into()),
            Message::WalletTrackerRefreshOrders(ADDRESS.into()),
            Message::WalletTrackerLoaded(
                ADDRESS.into(),
                read_context,
                Err("tracker failed".to_string()).into(),
            ),
            Message::WalletTrackerBatchLoaded(
                read_context,
                vec![(ADDRESS.to_string(), Err("batch failed".to_string()))].into(),
            ),
            Message::WalletTrackerOrdersLoaded(
                ADDRESS.into(),
                read_context,
                Err("orders failed".to_string()).into(),
            ),
            Message::PortfolioLoaded(
                ADDRESS.into(),
                1,
                Err("portfolio failed".to_string()).into(),
            ),
            Message::IncomeLoaded(ADDRESS.into(), 1, Err("income failed".to_string()).into()),
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
                            spot_balances_revision: 1,
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
                result: Err("quick failed".to_string()).into(),
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
                result: Err("leverage failed".to_string()).into(),
            },
            Message::CancelResult {
                request_id: 1,
                account_address: ADDRESS.into(),
                pending_indicator_id: None,
                result: Err("cancel failed".to_string()).into(),
            },
            Message::CancelOrderStatusLoaded {
                request_id: 1,
                account_address: ADDRESS.into(),
                oid: 42.into(),
                symbol: "HYPE".into(),
                result: Err("status failed".to_string()).into(),
            },
            Message::MoveOrderModifyResult {
                request_id: 2,
                account_address: ADDRESS.into(),
                coin: "HYPE".into(),
                oid: 42.into(),
                pending_indicator_id: None,
                result: Err("modify failed".to_string()).into(),
            },
            Message::MoveOrderStatusLoaded {
                request_id: 2,
                account_address: ADDRESS.into(),
                coin: "HYPE".into(),
                oid: 42.into(),
                result: Err("move status failed".to_string()).into(),
            },
            Message::WalletAddressInputChanged(ADDRESS.into()),
            Message::AddAccountAddressChanged(ADDRESS.into()),
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
                Err("account failed".to_string()).into(),
            ),
            Message::RetryTwapReconciliationAccountData(ADDRESS.into()),
            Message::WsUserDataUpdate(
                WsUserDataStreamParams::new(Some(ADDRESS.to_string()), Vec::new())
                    .with_generation(1),
                Some(ADDRESS.into()),
                Box::new(WsUserData::Lagged { skipped: 1 }),
            ),
        ];

        for message in messages {
            let rendered = format!("{message:?}");
            assert!(rendered.contains("<redacted>"), "{rendered}");
            assert!(!rendered.contains(ADDRESS), "{rendered}");
            assert!(!rendered.contains(ACCOUNT_KEY), "{rendered}");
            assert!(!rendered.contains(JOURNAL_ERROR), "{rendered}");
            assert!(!rendered.contains(WALLET_LABEL), "{rendered}");
        }
    }
}
