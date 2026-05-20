use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::helpers::{format_decimal_with_commas, format_price};
use crate::signing::OrderKind;

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
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AlfredTradeDraft {
    pub(crate) side: Option<AlfredTradeSide>,
    pub(crate) symbol_key: Option<String>,
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

#[derive(Debug, Clone, PartialEq)]
struct ParsedTradeIntent {
    side: Option<AlfredTradeSide>,
    amount: Option<f64>,
    amount_is_usd: bool,
    symbol: Option<String>,
    explicit_chase: bool,
    explicit_limit: bool,
    limit_price: Option<f64>,
    error: Option<String>,
}

impl ParsedTradeIntent {
    fn order_kind(&self) -> OrderKind {
        if self.explicit_chase {
            OrderKind::Chase
        } else if self.explicit_limit || self.limit_price.is_some() {
            OrderKind::Limit
        } else {
            OrderKind::Market
        }
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
        let symbol_display = resolved_symbol
            .map(Self::exchange_symbol_display_name)
            .or_else(|| intent.symbol.clone())
            .unwrap_or_else(|| "symbol".to_string());
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

fn parse_trade_intent(query: &str) -> Option<ParsedTradeIntent> {
    let tokens = trade_tokens(query);
    if tokens.is_empty() {
        return None;
    }

    let mut side = None;
    let mut amount = None;
    let mut amount_is_usd = false;
    let mut symbol = None;
    let mut explicit_chase = false;
    let mut explicit_limit = false;
    let mut explicit_market = false;
    let mut limit_price = None;
    let mut error = None;
    let mut consumed = vec![false; tokens.len()];

    let mut index = 0;
    while index < tokens.len() {
        let lower = tokens[index].to_ascii_lowercase();
        match lower.as_str() {
            "buy" | "long" | "bid" => {
                side = Some(AlfredTradeSide::Buy);
                consumed[index] = true;
            }
            "sell" | "short" | "ask" | "offer" => {
                side = Some(AlfredTradeSide::Sell);
                consumed[index] = true;
            }
            "chase" | "chasing" => {
                explicit_chase = true;
                consumed[index] = true;
            }
            "limit" => {
                explicit_limit = true;
                consumed[index] = true;
            }
            "market" => {
                explicit_market = true;
                consumed[index] = true;
            }
            "at" => {
                explicit_limit = true;
                consumed[index] = true;
                if let Some(next) = tokens.get(index + 1)
                    && let Some((price, _)) = parse_compact_amount(next)
                {
                    limit_price = Some(price);
                    consumed[index + 1] = true;
                    index += 1;
                }
            }
            _ => {}
        }
        index += 1;
    }

    for (idx, token) in tokens.iter().enumerate() {
        if consumed[idx] {
            continue;
        }
        if amount.is_none()
            && let Some((value, is_usd)) = parse_compact_amount(token)
        {
            amount = Some(value);
            amount_is_usd = is_usd;
            consumed[idx] = true;
        }
    }

    for (idx, token) in tokens.iter().enumerate() {
        if consumed[idx] || is_trade_filler(token) {
            continue;
        }
        symbol = Some(token.to_string());
        break;
    }

    if explicit_chase && (explicit_market || explicit_limit || limit_price.is_some()) {
        error = Some("Chase orders do not take a market, limit, or price modifier".to_string());
    }

    let looks_like_trade = side.is_some()
        || explicit_limit
        || explicit_market
        || (explicit_chase && (side.is_some() || amount.is_some() || symbol.is_some()))
        || (amount.is_some() && symbol.is_some());
    looks_like_trade.then_some(ParsedTradeIntent {
        side,
        amount,
        amount_is_usd,
        symbol,
        explicit_chase,
        explicit_limit,
        limit_price,
        error,
    })
}

fn trade_tokens(query: &str) -> Vec<String> {
    let raw: Vec<String> = query
        .split_whitespace()
        .map(trim_trade_token)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect();

    let mut tokens = Vec::with_capacity(raw.len());
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == "$"
            && let Some(next) = raw.get(index + 1)
        {
            tokens.push(format!("${next}"));
            index += 2;
            continue;
        }
        tokens.push(raw[index].clone());
        index += 1;
    }
    tokens
}

fn trim_trade_token(token: &str) -> &str {
    token.trim_matches(|ch: char| {
        matches!(
            ch,
            '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ';' | ','
        )
    })
}

fn is_trade_filler(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "a" | "an" | "the" | "of" | "for" | "order" | "trade" | "with" | "on"
    )
}

fn parse_compact_amount(token: &str) -> Option<(f64, bool)> {
    let token = token.trim();
    if token.is_empty() || token.starts_with('@') || token.starts_with('#') {
        return None;
    }

    let (is_usd, number) = token
        .strip_prefix('$')
        .map_or((false, token), |number| (true, number));
    let (number, multiplier) = match number.chars().last()?.to_ascii_lowercase() {
        'k' => (&number[..number.len() - 1], 1_000.0),
        'm' => (&number[..number.len() - 1], 1_000_000.0),
        'b' => (&number[..number.len() - 1], 1_000_000_000.0),
        _ => (number, 1.0),
    };
    if number.is_empty() {
        return None;
    }

    let value = number.replace(',', "").parse::<f64>().ok()? * multiplier;
    (value.is_finite() && value > 0.0).then_some((value, is_usd))
}

fn normalize_symbol_input(symbol: &str) -> String {
    if symbol.starts_with('@') || symbol.starts_with('#') || symbol.starts_with('+') {
        return symbol.to_string();
    }

    if let Some((dex, ticker)) = symbol.split_once(':') {
        format!(
            "{}:{}",
            dex.to_ascii_lowercase(),
            ticker.to_ascii_uppercase()
        )
    } else {
        symbol.to_ascii_uppercase()
    }
}

fn trade_amount_label(value: f64, is_usd: bool) -> String {
    let amount = display_amount(value);
    if is_usd { format!("${amount}") } else { amount }
}

fn display_amount(value: f64) -> String {
    let formatted = format_decimal_with_commas(value, 4);
    trim_decimal_zeros(formatted)
}

fn plain_amount(value: f64) -> String {
    trim_decimal_zeros(format!("{value:.8}"))
}

fn trim_decimal_zeros(mut value: String) -> String {
    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
    }
    value
}

fn trade_title(
    side: Option<AlfredTradeSide>,
    quantity: &str,
    symbol: &str,
    order_kind: OrderKind,
    price: Option<&str>,
) -> String {
    let side = match order_kind {
        OrderKind::Chase => side
            .map(|side| format!("CHASE {}", side.label()))
            .unwrap_or_else(|| "CHASE".to_string()),
        OrderKind::Market | OrderKind::Limit | OrderKind::LimitIoc | OrderKind::Twap => side
            .map(|side| side.label().to_string())
            .unwrap_or_else(|| "ORDER".to_string()),
    };
    let mut title = format!("{side} {quantity} {}", symbol.to_ascii_uppercase());
    if order_kind == OrderKind::Limit
        && let Some(price) = price
    {
        title.push_str(" @ ");
        title.push_str(price);
    }
    title
}

fn trade_detail(order_kind: OrderKind, quantity_is_usd: bool) -> String {
    let quantity = if quantity_is_usd {
        "USD notional"
    } else {
        "coin size"
    };
    match order_kind {
        OrderKind::Limit => format!("Limit order, {quantity}"),
        OrderKind::Market => format!("Market order, {quantity}"),
        OrderKind::Chase => format!("Chase order, {quantity}"),
        OrderKind::LimitIoc | OrderKind::Twap => "Trade draft".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::MarketType;

    fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 5,
            max_leverage: 50,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    fn terminal_with_hype() -> TradingTerminal {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![symbol("HYPE", MarketType::Perp)];
        terminal
    }

    #[test]
    fn parses_coin_market_order() {
        let intent = parse_trade_intent("buy 1k HYPE").expect("trade intent");

        assert_eq!(intent.side, Some(AlfredTradeSide::Buy));
        assert_eq!(intent.amount, Some(1_000.0));
        assert!(!intent.amount_is_usd);
        assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
        assert_eq!(intent.order_kind(), OrderKind::Market);
    }

    #[test]
    fn parses_usd_market_order_without_side_as_draft() {
        let intent = parse_trade_intent("$1k hype").expect("trade intent");

        assert_eq!(intent.side, None);
        assert_eq!(intent.amount, Some(1_000.0));
        assert!(intent.amount_is_usd);
        assert_eq!(intent.symbol.as_deref(), Some("hype"));
        assert_eq!(intent.order_kind(), OrderKind::Market);
    }

    #[test]
    fn parses_usd_limit_order() {
        let intent = parse_trade_intent("buy $1k hype at 43").expect("trade intent");

        assert_eq!(intent.side, Some(AlfredTradeSide::Buy));
        assert_eq!(intent.amount, Some(1_000.0));
        assert!(intent.amount_is_usd);
        assert_eq!(intent.symbol.as_deref(), Some("hype"));
        assert_eq!(intent.limit_price, Some(43.0));
        assert_eq!(intent.order_kind(), OrderKind::Limit);
    }

    #[test]
    fn parses_coin_chase_order_without_side() {
        let intent = parse_trade_intent("chase 1k HYPE").expect("trade intent");

        assert_eq!(intent.side, None);
        assert_eq!(intent.amount, Some(1_000.0));
        assert!(!intent.amount_is_usd);
        assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
        assert_eq!(intent.order_kind(), OrderKind::Chase);
    }

    #[test]
    fn parses_usd_chase_order_without_side() {
        let intent = parse_trade_intent("chase $1k hype").expect("trade intent");

        assert_eq!(intent.side, None);
        assert_eq!(intent.amount, Some(1_000.0));
        assert!(intent.amount_is_usd);
        assert_eq!(intent.symbol.as_deref(), Some("hype"));
        assert_eq!(intent.order_kind(), OrderKind::Chase);
    }

    #[test]
    fn parses_chase_order_with_side_before_or_after_keyword() {
        let buy = parse_trade_intent("buy chase $1k HYPE").expect("buy chase intent");
        let sell = parse_trade_intent("chase sell 250 HYPE").expect("sell chase intent");

        assert_eq!(buy.side, Some(AlfredTradeSide::Buy));
        assert_eq!(buy.order_kind(), OrderKind::Chase);
        assert_eq!(sell.side, Some(AlfredTradeSide::Sell));
        assert_eq!(sell.order_kind(), OrderKind::Chase);
    }

    #[test]
    fn chase_draft_without_side_can_be_applied() {
        let terminal = terminal_with_hype();
        let draft = terminal
            .alfred_trade_draft("chase $1k HYPE")
            .expect("chase draft");

        assert_eq!(draft.side, None);
        assert_eq!(draft.symbol_key.as_deref(), Some("HYPE"));
        assert_eq!(draft.order_kind, OrderKind::Chase);
        assert_eq!(draft.title, "CHASE $1,000 HYPE");
        assert_eq!(draft.detail, "Chase order, USD notional");
        assert_eq!(draft.tag, "Chase");
        assert!(draft.can_submit());
    }

    #[test]
    fn rejects_chase_price_modifiers() {
        let intent = parse_trade_intent("chase $1k HYPE at 43").expect("trade intent");

        assert_eq!(intent.order_kind(), OrderKind::Chase);
        assert_eq!(
            intent.error.as_deref(),
            Some("Chase orders do not take a market, limit, or price modifier")
        );
    }

    #[test]
    fn ignores_non_trade_queries() {
        assert_eq!(parse_trade_intent("portfolio pane"), None);
        assert_eq!(parse_trade_intent("hype"), None);
        assert_eq!(parse_trade_intent("chase"), None);
    }
}
