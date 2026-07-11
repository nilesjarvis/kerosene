use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::signing::OrderKind;

use std::fmt;

mod display;
mod parse;

use display::{plain_amount, trade_amount_label, trade_detail, trade_title};
use parse::*;

// ---------------------------------------------------------------------------
// Natural Language Trading
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlfredTradeSide {
    Buy,
    Sell,
}

impl AlfredTradeSide {
    pub(crate) fn is_buy(self) -> bool {
        self == Self::Buy
    }

    fn label(self) -> &'static str {
        match self {
            Self::Buy => "↑ BUY",
            Self::Sell => "↓ SELL",
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct AlfredTradeDraft {
    pub(crate) side: Option<AlfredTradeSide>,
    pub(crate) symbol_key: Option<String>,
    pub(crate) icon_symbol: Option<String>,
    pub(crate) icon_title_anchor: Option<String>,
    pub(crate) quantity: Option<f64>,
    pub(crate) quantity_is_usd: bool,
    pub(crate) order_kind: OrderKind,
    pub(crate) limit_price: Option<f64>,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) tag: String,
    pub(crate) error: Option<String>,
}

impl fmt::Debug for AlfredTradeDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlfredTradeDraft")
            .field("side", &self.side)
            .field("has_symbol_key", &self.symbol_key.is_some())
            .field("has_icon_symbol", &self.icon_symbol.is_some())
            .field("has_icon_title_anchor", &self.icon_title_anchor.is_some())
            .field("quantity", &self.quantity.as_ref().map(|_| "<redacted>"))
            .field("quantity_is_usd", &self.quantity_is_usd)
            .field("order_kind", &self.order_kind)
            .field(
                "limit_price",
                &self.limit_price.as_ref().map(|_| "<redacted>"),
            )
            .field("title", &"<redacted>")
            .field("detail", &"<redacted>")
            .field("tag", &"<redacted>")
            .field("has_error", &self.error.is_some())
            .finish()
    }
}

impl AlfredTradeDraft {
    pub(crate) fn can_submit(&self) -> bool {
        self.error.is_none()
            && self.symbol_key.is_some()
            && self.quantity.is_some()
            && (self.side.is_some() || self.order_kind == OrderKind::Chase)
    }

    pub(crate) fn quantity_input(&self) -> String {
        plain_amount(self.quantity.unwrap_or_default())
    }

    pub(crate) fn limit_price_input(&self) -> Option<String> {
        self.limit_price.map(plain_amount)
    }
}

impl TradingTerminal {
    pub(crate) fn alfred_trade_draft(&self, query: &str) -> Option<AlfredTradeDraft> {
        let intent = parse_trade_intent(query)?;
        Some(self.resolve_trade_draft(intent))
    }

    fn resolve_trade_draft(&self, intent: ParsedTradeIntent) -> AlfredTradeDraft {
        let order_kind = intent.order_kind();
        let mut error = intent.error.clone();

        let resolved_symbol = match intent.symbol.as_deref() {
            Some(_symbol) if self.exchange_symbols.is_empty() && error.is_none() => {
                error = Some("Symbols are still loading".to_string());
                None
            }
            Some(symbol) => match self.resolve_trade_symbol(symbol, intent.explicit_spot) {
                Ok(Some(symbol)) => {
                    if error.is_none()
                        && let Err(message) =
                            self.validate_exchange_symbol_orderable(symbol, "Trade")
                    {
                        error = Some(message);
                    }
                    Some(symbol)
                }
                Ok(None) => {
                    if error.is_none() {
                        error = Some(unresolved_trade_symbol_error(symbol, intent.explicit_spot));
                    }
                    None
                }
                Err(message) => {
                    if error.is_none() {
                        error = Some(message);
                    }
                    None
                }
            },
            None if error.is_none() => {
                error = Some("Add a symbol".to_string());
                None
            }
            None => None,
        };

        if intent.amount.is_none() && error.is_none() {
            error = Some("Add an order size".to_string());
        }
        if intent.side.is_none() && order_kind != OrderKind::Chase && error.is_none() {
            error = Some("Start with buy or sell".to_string());
        }
        if order_kind == OrderKind::Limit && intent.limit_price.is_none() && error.is_none() {
            error = Some("Add a limit price".to_string());
        }

        let symbol_key = resolved_symbol.map(|symbol| symbol.key.clone());
        let icon_symbol = symbol_key.clone().or_else(|| intent.symbol.clone());
        let symbol_display = resolved_symbol
            .map(Self::exchange_symbol_display_name)
            .or_else(|| intent.symbol.clone())
            .unwrap_or_else(|| "symbol".to_string());
        let icon_title_anchor = Some(symbol_display.to_ascii_uppercase());
        let quantity_label = intent
            .amount
            .map(|amount| trade_amount_label(amount, intent.amount_is_usd))
            .unwrap_or_else(|| "size".to_string());
        let price_label = intent.limit_price.map(format_price);
        let title = trade_title(
            intent.side,
            &quantity_label,
            &symbol_display,
            order_kind,
            price_label.as_deref(),
        );
        let detail = error
            .clone()
            .unwrap_or_else(|| trade_detail(order_kind, intent.amount_is_usd));
        let tag = match order_kind {
            OrderKind::Limit => "Limit",
            OrderKind::Market => "Market",
            OrderKind::Chase => "Chase",
            OrderKind::LimitIoc | OrderKind::Twap => "Trade",
        }
        .to_string();

        AlfredTradeDraft {
            side: intent.side,
            symbol_key,
            icon_symbol,
            icon_title_anchor,
            quantity: intent.amount,
            quantity_is_usd: intent.amount_is_usd,
            order_kind,
            limit_price: intent.limit_price,
            title,
            detail,
            tag,
            error,
        }
    }

    /// Bare tickers resolve perp-first (matching the quick symbol search and
    /// `switch_active_symbol_internal`); the spot market must be requested
    /// explicitly, either by its pair spelling ("HYPE/USDC") or with a `spot`
    /// qualifier token. An explicit spot request never falls back to the perp.
    fn resolve_trade_symbol(
        &self,
        raw_symbol: &str,
        spot_requested: bool,
    ) -> Result<Option<&ExchangeSymbol>, String> {
        if raw_symbol.contains('/') {
            return Ok(self.resolve_spot_pair_symbol(raw_symbol));
        }

        let normalized = normalize_symbol_input(raw_symbol);
        if !spot_requested {
            // An exact indexed key is an explicit spot identity, but a bare
            // ticker must never fall through to the first spot market. The
            // same base token can trade against multiple quote tokens.
            if let Some(symbol) = self.exchange_symbols.iter().find(|symbol| {
                (symbol.key.eq_ignore_ascii_case(raw_symbol)
                    || symbol.key.eq_ignore_ascii_case(&normalized))
                    && (symbol.market_type != MarketType::Spot || symbol.key.starts_with('@'))
            }) {
                return Ok(Some(symbol));
            }
            if let Some(symbol) = self.exchange_symbols.iter().find(|symbol| {
                symbol.market_type == MarketType::Perp
                    && symbol.ticker.eq_ignore_ascii_case(&normalized)
            }) {
                return Ok(Some(symbol));
            }

            let mut spot_names = self
                .exchange_symbols
                .iter()
                .filter(|symbol| {
                    symbol.market_type == MarketType::Spot
                        && symbol.ticker.eq_ignore_ascii_case(&normalized)
                })
                .map(Self::exchange_symbol_display_name)
                .collect::<Vec<_>>();
            if !spot_names.is_empty() {
                spot_names.sort_unstable();
                spot_names.dedup();
                return Err(if spot_names.len() == 1 {
                    format!(
                        "'{}' is a spot market; add 'spot' or use {}",
                        normalized.to_ascii_uppercase(),
                        spot_names[0]
                    )
                } else {
                    format!(
                        "Multiple spot markets for '{}'; use an explicit pair such as {}",
                        normalized.to_ascii_uppercase(),
                        spot_names.join(" or ")
                    )
                });
            }
            return Ok(None);
        }

        if let Some(symbol) = self.exchange_symbols.iter().find(|symbol| {
            symbol.market_type == MarketType::Spot
                && (symbol.key.eq_ignore_ascii_case(raw_symbol)
                    || symbol.key.eq_ignore_ascii_case(&normalized))
        }) {
            return Ok(Some(symbol));
        }

        let mut matching_pairs = self.exchange_symbols.iter().filter(|symbol| {
            symbol.market_type == MarketType::Spot
                && symbol.ticker.eq_ignore_ascii_case(&normalized)
        });
        let first = matching_pairs.next();
        if let Some(second) = matching_pairs.next() {
            let mut names = vec![
                first
                    .map(Self::exchange_symbol_display_name)
                    .unwrap_or_else(|| normalized.clone()),
                Self::exchange_symbol_display_name(second),
            ];
            names.extend(matching_pairs.map(Self::exchange_symbol_display_name));
            names.sort_unstable();
            names.dedup();
            return Err(format!(
                "Multiple spot markets for '{}'; use a pair such as {}",
                normalized.to_ascii_uppercase(),
                names.join(" or ")
            ));
        }

        Ok(first)
    }

    fn resolve_spot_pair_symbol(&self, pair: &str) -> Option<&ExchangeSymbol> {
        self.exchange_symbols.iter().find(|symbol| {
            symbol.market_type == MarketType::Spot
                && symbol
                    .display_name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(pair))
        })
    }
}

fn unresolved_trade_symbol_error(raw_symbol: &str, spot_requested: bool) -> String {
    let symbol = raw_symbol.to_ascii_uppercase();
    if spot_requested || raw_symbol.contains('/') {
        format!("No spot market for '{symbol}'")
    } else {
        format!("Unknown symbol '{symbol}'")
    }
}

#[cfg(test)]
mod tests;
