use crate::signing::OrderKind;

use super::AlfredTradeSide;

// ---------------------------------------------------------------------------
// Trade Intent Parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ParsedTradeIntent {
    pub(super) side: Option<AlfredTradeSide>,
    pub(super) amount: Option<f64>,
    pub(super) amount_is_usd: bool,
    pub(super) symbol: Option<String>,
    pub(super) explicit_spot: bool,
    pub(super) explicit_chase: bool,
    pub(super) explicit_limit: bool,
    pub(super) limit_price: Option<f64>,
    pub(super) error: Option<String>,
}

impl ParsedTradeIntent {
    pub(super) fn order_kind(&self) -> OrderKind {
        if self.explicit_chase {
            OrderKind::Chase
        } else if self.explicit_limit || self.limit_price.is_some() {
            OrderKind::Limit
        } else {
            OrderKind::Market
        }
    }
}

pub(super) fn parse_trade_intent(query: &str) -> Option<ParsedTradeIntent> {
    let tokens = trade_tokens(query);
    if tokens.is_empty() {
        return None;
    }

    let mut side = None;
    let mut amount = None;
    let mut amount_is_usd = false;
    let mut symbol = None;
    let mut explicit_spot = false;
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
            "spot" => {
                explicit_spot = true;
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
        explicit_spot,
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

pub(super) fn normalize_symbol_input(symbol: &str) -> String {
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
