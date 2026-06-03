use super::results::result_requires_account_refresh;
use crate::account::AccountDataFetchScope;
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::PendingLeverageUpdateContext;
use crate::signing::{ExchangeResponse, update_leverage};

use iced::Task;

const DEFAULT_ORDER_LEVERAGE: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrderLeverageConstraints {
    max_leverage: u32,
    cross_allowed: bool,
}

impl TradingTerminal {
    pub(crate) fn handle_order_leverage_input_changed(&mut self, value: String) {
        self.order_leverage_input = sanitize_leverage_input(&value);
    }

    pub(crate) fn handle_set_order_leverage_cross(&mut self, is_cross: bool) {
        if is_cross
            && self
                .active_order_leverage_constraints()
                .is_some_and(|(_, cross_allowed)| !cross_allowed)
        {
            self.order_leverage_is_cross = false;
            self.order_status = Some((
                format!(
                    "{} only supports isolated margin",
                    self.active_symbol_display.to_uppercase()
                ),
                true,
            ));
            return;
        }

        self.order_leverage_is_cross = is_cross;
    }

    pub(crate) fn submit_order_leverage_update(&mut self) -> Task<Message> {
        if self.pending_leverage_update.is_some() {
            return Task::none();
        }

        let key = self.wallet_key_input.trim().to_string();
        let Some(address) = self.connected_address.clone() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        };
        if key.is_empty() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let Some(symbol) = self.active_order_leverage_symbol().cloned() else {
            self.order_status = Some((
                "Leverage is only available for perpetual markets".into(),
                true,
            ));
            return Task::none();
        };

        let constraints = order_leverage_constraints_for_symbol(&symbol);
        let Some(leverage) = parse_leverage_input(&self.order_leverage_input) else {
            self.order_status = Some(("Enter leverage as a whole number".into(), true));
            return Task::none();
        };
        if leverage > constraints.max_leverage {
            self.order_status = Some((
                format!(
                    "Max leverage for {} is {}x",
                    Self::exchange_symbol_display_name(&symbol).to_uppercase(),
                    constraints.max_leverage
                ),
                true,
            ));
            return Task::none();
        }

        let is_cross = self.order_leverage_is_cross && constraints.cross_allowed;
        if self.order_leverage_is_cross && !constraints.cross_allowed {
            self.order_leverage_is_cross = false;
            self.order_status = Some((
                format!(
                    "{} only supports isolated margin",
                    Self::exchange_symbol_display_name(&symbol).to_uppercase()
                ),
                true,
            ));
            return Task::none();
        }

        let context = PendingLeverageUpdateContext {
            address,
            symbol_key: symbol.key.clone(),
            display: Self::exchange_symbol_display_name(&symbol),
            asset: symbol.asset_index,
            dex: symbol
                .key
                .split_once(':')
                .map(|(dex, _)| dex.to_ascii_lowercase()),
            is_cross,
            leverage,
        };

        self.pending_leverage_update = Some(context.clone());
        self.order_status = Some(("Updating leverage...".into(), false));

        Task::perform(
            update_leverage(
                key.into(),
                context.asset,
                context.is_cross,
                context.leverage,
            ),
            move |result| Message::OrderLeverageResult {
                context: context.clone(),
                result: Box::new(result),
            },
        )
    }

    pub(crate) fn handle_order_leverage_result(
        &mut self,
        context: PendingLeverageUpdateContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        if self.pending_leverage_update.as_ref() != Some(&context) {
            return Task::none();
        }
        self.pending_leverage_update = None;

        let account_still_current =
            self.connected_address.as_deref() == Some(context.address.as_str());
        if !account_still_current {
            return Task::none();
        }

        let should_refresh = result_requires_account_refresh(&result);

        match result {
            Ok(response) if !response.is_error() => {
                if self.active_symbol == context.symbol_key {
                    self.order_leverage_input = context.leverage.to_string();
                    self.order_leverage_is_cross = context.is_cross;
                }
                self.order_status = Some((
                    format!(
                        "{} leverage updated: {} {}x",
                        context.display.to_uppercase(),
                        context.margin_mode_label(),
                        context.leverage
                    ),
                    false,
                ));
            }
            Ok(response) => {
                self.order_status = Some((response.summary(), true));
            }
            Err(error) => {
                self.order_status = Some((error, true));
            }
        }

        if should_refresh {
            let scope = context
                .dex
                .as_deref()
                .map(AccountDataFetchScope::hip3_dex)
                .unwrap_or_else(|| self.account_data_fetch_scope());
            self.force_refresh_account_data_with_scope(context.address, scope)
        } else {
            Task::none()
        }
    }

    pub(crate) fn sync_order_leverage_form_for_active_symbol(&mut self) {
        let Some(symbol) = self.active_order_leverage_symbol() else {
            self.order_leverage_input = DEFAULT_ORDER_LEVERAGE.to_string();
            self.order_leverage_is_cross = false;
            return;
        };

        let symbol_key = symbol.key.clone();
        let constraints = order_leverage_constraints_for_symbol(symbol);
        let account_setting = self
            .account_data
            .as_ref()
            .and_then(|data| data.get_leverage_for(&symbol_key, &self.exchange_symbols))
            .filter(|(_, _, is_actual)| *is_actual);
        let existing = parse_leverage_input(&self.order_leverage_input)
            .unwrap_or(DEFAULT_ORDER_LEVERAGE)
            .clamp(DEFAULT_ORDER_LEVERAGE, constraints.max_leverage);
        let leverage = account_setting
            .map(|(_, leverage, _)| leverage)
            .unwrap_or(existing)
            .clamp(DEFAULT_ORDER_LEVERAGE, constraints.max_leverage);
        let is_cross = account_setting
            .map(|(is_cross, _, _)| is_cross)
            .unwrap_or(constraints.cross_allowed)
            && constraints.cross_allowed;

        self.order_leverage_input = leverage.to_string();
        self.order_leverage_is_cross = is_cross;
    }

    pub(crate) fn active_order_leverage_constraints(&self) -> Option<(u32, bool)> {
        self.active_order_leverage_symbol()
            .map(order_leverage_constraints_for_symbol)
            .map(|constraints| (constraints.max_leverage, constraints.cross_allowed))
    }

    fn active_order_leverage_symbol(&self) -> Option<&ExchangeSymbol> {
        let symbol = self.resolve_exchange_symbol_by_key_or_ticker(&self.active_symbol)?;
        (symbol.market_type == MarketType::Perp && self.exchange_symbol_is_orderable(symbol))
            .then_some(symbol)
    }
}

fn order_leverage_constraints_for_symbol(symbol: &ExchangeSymbol) -> OrderLeverageConstraints {
    OrderLeverageConstraints {
        max_leverage: symbol.max_leverage.max(DEFAULT_ORDER_LEVERAGE),
        cross_allowed: !symbol.only_isolated,
    }
}

fn sanitize_leverage_input(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .take(3)
        .collect()
}

fn parse_leverage_input(value: &str) -> Option<u32> {
    let leverage = value.trim().parse::<u32>().ok()?;
    (leverage >= DEFAULT_ORDER_LEVERAGE).then_some(leverage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::MarketType;

    fn symbol(key: &str, max_leverage: u32, only_isolated: bool) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.split(':').nth(1).unwrap_or(key).to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage,
            only_isolated,
            market_type: MarketType::Perp,
            outcome: None,
        }
    }

    #[test]
    fn leverage_input_sanitizer_keeps_digits_only() {
        assert_eq!(sanitize_leverage_input(" 12x "), "12");
        assert_eq!(sanitize_leverage_input("abc"), "");
        assert_eq!(sanitize_leverage_input("1234"), "123");
    }

    #[test]
    fn leverage_input_parser_requires_positive_integer() {
        assert_eq!(parse_leverage_input("1"), Some(1));
        assert_eq!(parse_leverage_input("50"), Some(50));
        assert_eq!(parse_leverage_input("0"), None);
        assert_eq!(parse_leverage_input("1.5"), None);
    }

    #[test]
    fn isolated_only_symbol_disallows_cross() {
        let constraints = order_leverage_constraints_for_symbol(&symbol("xyz:NVDA", 10, true));

        assert_eq!(
            constraints,
            OrderLeverageConstraints {
                max_leverage: 10,
                cross_allowed: false,
            }
        );
    }

    #[test]
    fn leverage_constraints_never_expose_zero_max() {
        let constraints = order_leverage_constraints_for_symbol(&symbol("BTC", 0, false));

        assert_eq!(constraints.max_leverage, 1);
        assert!(constraints.cross_allowed);
    }

    fn pending_context(
        symbol_key: &str,
        is_cross: bool,
        leverage: u32,
    ) -> PendingLeverageUpdateContext {
        PendingLeverageUpdateContext {
            address: "0xabc".to_string(),
            symbol_key: symbol_key.to_string(),
            display: symbol_key.to_string(),
            asset: 0,
            dex: symbol_key
                .split_once(':')
                .map(|(dex, _)| dex.to_ascii_lowercase()),
            is_cross,
            leverage,
        }
    }

    fn ok_exchange_response() -> ExchangeResponse {
        serde_json::from_str(r#"{"status":"ok","response":{"type":"default"}}"#)
            .expect("valid exchange response")
    }

    #[test]
    fn leverage_result_uses_submitted_context_not_current_form() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        terminal.active_symbol = "BTC".to_string();
        terminal.order_leverage_input = "99".to_string();
        terminal.order_leverage_is_cross = false;
        let context = pending_context("BTC", true, 12);
        terminal.pending_leverage_update = Some(context.clone());

        let _ = terminal.handle_order_leverage_result(context, Ok(ok_exchange_response()));

        assert_eq!(terminal.pending_leverage_update, None);
        assert_eq!(terminal.order_leverage_input, "12");
        assert!(terminal.order_leverage_is_cross);
        assert_eq!(
            terminal.order_status,
            Some(("BTC leverage updated: Cross 12x".to_string(), false))
        );
    }

    #[test]
    fn leverage_result_ignores_stale_context() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.connected_address = Some("0xabc".to_string());
        let current = pending_context("ETH", false, 3);
        terminal.pending_leverage_update = Some(current.clone());

        let _ = terminal.handle_order_leverage_result(
            pending_context("BTC", true, 12),
            Ok(ok_exchange_response()),
        );

        assert_eq!(terminal.pending_leverage_update, Some(current));
        assert_eq!(terminal.order_status, None);
    }
}
