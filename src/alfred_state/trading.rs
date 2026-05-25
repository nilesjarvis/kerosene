use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::signing::OrderKind;

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

#[derive(Debug, Clone, PartialEq)]
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
            Some(symbol) => match self.resolve_trade_symbol(symbol) {
                Some(symbol) => {
                    if error.is_none()
                        && let Err(message) =
                            self.validate_exchange_symbol_orderable(symbol, "Trade")
                    {
                        error = Some(message);
                    }
                    Some(symbol)
                }
                None => {
                    if error.is_none() {
                        error = Some(format!("Unknown symbol '{}'", symbol.to_ascii_uppercase()));
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

    fn resolve_trade_symbol(&self, raw_symbol: &str) -> Option<&ExchangeSymbol> {
        let normalized = normalize_symbol_input(raw_symbol);
        self.resolve_exchange_symbol_by_key_or_ticker(raw_symbol)
            .or_else(|| self.resolve_exchange_symbol_by_key_or_ticker(&normalized))
    }
}

#[cfg(test)]
mod tests;
