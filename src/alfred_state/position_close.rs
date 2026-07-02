use crate::account::Position;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{parse_finite_number, trim_decimal_zeros};

mod parse;

use parse::{ParsedClosePositionIntent, parse_close_position_intent};

// ---------------------------------------------------------------------------
// Natural Language Position Close
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AlfredClosePositionDraft {
    pub(crate) coin: Option<String>,
    pub(crate) fraction: f64,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) tag: String,
    pub(crate) error: Option<String>,
}

impl AlfredClosePositionDraft {
    pub(crate) fn can_submit(&self) -> bool {
        self.error.is_none() && self.coin.is_some() && self.fraction > 0.0
    }
}

impl TradingTerminal {
    pub(crate) fn alfred_close_position_draft(
        &self,
        query: &str,
    ) -> Option<AlfredClosePositionDraft> {
        let intent = parse_close_position_intent(query)?;
        Some(self.resolve_close_position_draft(intent))
    }

    fn resolve_close_position_draft(
        &self,
        intent: ParsedClosePositionIntent,
    ) -> AlfredClosePositionDraft {
        let fraction = intent.fraction.unwrap_or(1.0);
        let percent_label = close_percent_label(fraction);
        let mut error = intent.error;
        let mut resolved_position = None;

        match intent.symbol.as_deref() {
            Some(symbol) if error.is_none() => {
                if self.account_loading {
                    error = Some("Account refresh in progress".to_string());
                } else if self.connected_order_account_snapshot().is_none() {
                    error = Some("No account data available".to_string());
                } else if let Some((coin, position)) = self.resolve_close_position(symbol) {
                    resolved_position = Some((coin, position.clone()));
                } else if let Some(spot_error) = self.spot_holding_close_error(symbol) {
                    error = Some(spot_error);
                } else {
                    error = Some(format!(
                        "No open position for {}",
                        symbol.to_ascii_uppercase()
                    ));
                }
            }
            None if error.is_none() => {
                error = Some("Add a ticker to close".to_string());
            }
            _ => {}
        }

        let coin = resolved_position
            .as_ref()
            .map(|(coin, _)| coin.clone())
            .or_else(|| intent.symbol.clone());
        let title_symbol = coin.clone().unwrap_or_else(|| "ticker".to_string());
        let title = format!(
            "CLOSE {percent_label} {}",
            title_symbol.to_ascii_uppercase()
        );
        let detail = match (&error, resolved_position.as_ref()) {
            (Some(error), _) => error.clone(),
            (None, Some((_, position))) => {
                let side = close_position_side_label(&position.szi);
                format!("Market close {percent_label} of {side} position")
            }
            (None, None) => format!("Market close {percent_label} of position"),
        };

        AlfredClosePositionDraft {
            coin: resolved_position.map(|(coin, _)| coin),
            fraction,
            title,
            detail,
            tag: "Close".to_string(),
            error,
        }
    }

    fn resolve_close_position(&self, raw_symbol: &str) -> Option<(String, &Position)> {
        let positions = &self
            .connected_order_account_snapshot()?
            .1
            .clearinghouse
            .asset_positions;
        let normalized = normalize_close_symbol_input(raw_symbol);
        let resolved_key = self
            .resolve_exchange_symbol_by_key_or_ticker(raw_symbol)
            .map(|symbol| symbol.key.as_str())
            .or_else(|| {
                self.resolve_exchange_symbol_by_key_or_ticker(&normalized)
                    .map(|symbol| symbol.key.as_str())
            });

        positions
            .iter()
            .find(|ap| close_position_matches_symbol(&ap.position.coin, raw_symbol, &normalized))
            .or_else(|| {
                resolved_key.and_then(|key| positions.iter().find(|ap| ap.position.coin == key))
            })
            .map(|ap| (ap.position.coin.clone(), &ap.position))
    }

    /// `close` only targets perp positions, but the Positions tab shows spot
    /// holdings as positions too. When the ticker misses the perp
    /// clearinghouse yet matches a held spot balance, say so explicitly
    /// instead of denying a position the user can see.
    fn spot_holding_close_error(&self, raw_symbol: &str) -> Option<String> {
        let (_, data) = self.connected_order_account_snapshot()?;
        let ticker = self.close_symbol_spot_ticker(raw_symbol)?;
        let balance = data
            .spot
            .balances
            .iter()
            .find(|balance| balance.coin.eq_ignore_ascii_case(&ticker))?;
        let holding = self.spot_asset_position_for_balance(balance, &data.fills)?;

        let coin = balance.coin.clone();
        let pair = self.display_name_for_symbol(&holding.position.coin);
        let sellable = parse_finite_number(&balance.total)?
            - parse_finite_number(&balance.hold).unwrap_or(0.0);
        if sellable > 0.0 {
            let amount = trim_decimal_zeros(format!("{sellable:.8}"));
            Some(format!(
                "{coin} is a spot balance; close only closes perp positions — try 'sell {amount} {pair}'"
            ))
        } else {
            Some(format!(
                "{coin} is a spot balance; close only closes perp positions"
            ))
        }
    }

    /// Maps a typed close symbol to the spot balance ticker it names:
    /// "hype" and "HYPE/USDC" resolve to "HYPE", and a raw pair key like
    /// "@107" resolves through its spot market entry.
    fn close_symbol_spot_ticker(&self, raw_symbol: &str) -> Option<String> {
        let normalized = normalize_close_symbol_input(raw_symbol);
        let base = normalized.split('/').next().unwrap_or(&normalized);
        if base.starts_with('@') {
            return self
                .exchange_symbols
                .iter()
                .find(|symbol| symbol.key == base && symbol.market_type == MarketType::Spot)
                .map(|symbol| symbol.ticker.clone());
        }
        Some(base.to_string())
    }
}

fn normalize_close_symbol_input(symbol: &str) -> String {
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

fn close_position_matches_symbol(coin: &str, raw_symbol: &str, normalized: &str) -> bool {
    coin.eq_ignore_ascii_case(raw_symbol) || coin == normalized
}

fn close_percent_label(fraction: f64) -> String {
    let percent = fraction * 100.0;
    if (percent.fract()).abs() < f64::EPSILON {
        format!("{percent:.0}%")
    } else {
        format!("{percent:.2}%")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn close_position_side_label(raw_szi: &str) -> &'static str {
    match raw_szi.trim().parse::<f64>() {
        Ok(size) if size < 0.0 => "short",
        Ok(size) if size > 0.0 => "long",
        _ => "open",
    }
}

#[cfg(test)]
mod tests;
