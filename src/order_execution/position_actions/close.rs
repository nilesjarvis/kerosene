use super::super::pricing::{wire_market_price, wire_rounded_price};
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{finite_value, positive_finite_value};
use crate::message::Message;
use crate::signing::{OrderKind, float_to_wire, place_order};

use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosePositionInputError {
    InvalidPositionSize,
    InvalidFraction,
}

fn close_position_order_side_and_size(
    raw_szi: &str,
    fraction: f64,
) -> Result<(bool, String), ClosePositionInputError> {
    let Some(fraction) = positive_finite_value(fraction) else {
        return Err(ClosePositionInputError::InvalidFraction);
    };
    if fraction > 1.0 {
        return Err(ClosePositionInputError::InvalidFraction);
    }

    let szi = raw_szi
        .trim()
        .parse::<f64>()
        .map_err(|_| ClosePositionInputError::InvalidPositionSize)?;
    let Some(szi) = finite_value(szi) else {
        return Err(ClosePositionInputError::InvalidPositionSize);
    };
    if szi.abs() <= 1e-12 {
        return Err(ClosePositionInputError::InvalidPositionSize);
    }

    let is_buy = szi < 0.0;
    let close_size = szi.abs() * fraction;
    Ok((is_buy, float_to_wire(close_size)))
}

impl TradingTerminal {
    pub(crate) fn execute_close_position(
        &mut self,
        coin: &str,
        fraction: f64,
        use_market: bool,
    ) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }
        if self.symbol_key_is_hidden(coin) {
            self.order_status = Some(("Position ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }

        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before closing".into(),
                true,
            ));
            return Task::none();
        }

        let Some(account_data) = self.account_data.as_ref() else {
            self.order_status = Some((
                "No account data available; refresh before closing".into(),
                true,
            ));
            return Task::none();
        };
        let now_ms = Self::now_ms();
        if !account_data.is_fresh_for_position_action(now_ms) {
            let age_label = account_data
                .position_action_snapshot_age_ms(now_ms)
                .map(|age| format!("{}s old", age.div_ceil(1000)))
                .unwrap_or_else(|| "from the future".to_string());
            self.order_status = Some((
                format!("Account data is stale ({age_label}); refresh before closing positions"),
                true,
            ));
            return self.refresh_account_data();
        }

        let pos = account_data
            .clearinghouse
            .asset_positions
            .iter()
            .find(|ap| ap.position.coin == coin)
            .map(|ap| &ap.position);
        let Some(pos) = pos else {
            self.order_status = Some((format!("No position found for {coin}"), true));
            return Task::none();
        };

        let (is_buy, size) = match close_position_order_side_and_size(&pos.szi, fraction) {
            Ok(inputs) => inputs,
            Err(ClosePositionInputError::InvalidPositionSize) => {
                self.order_status = Some(("Position size is invalid".into(), true));
                return Task::none();
            }
            Err(ClosePositionInputError::InvalidFraction) => {
                self.order_status = Some(("Close fraction is invalid".into(), true));
                return Task::none();
            }
        };

        let sym = self.exchange_symbols.iter().find(|s| s.key == coin);
        let Some(sym) = sym else {
            self.order_status = Some((format!("Symbol '{coin}' not found"), true));
            return Task::none();
        };
        if sym.market_type == MarketType::Outcome {
            self.outcome_read_only_status("position closing");
            return Task::none();
        }
        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;

        let order_kind = if use_market {
            OrderKind::Market
        } else {
            OrderKind::Limit
        };

        let Some(mid) = self.resolve_mid_for_symbol(coin) else {
            self.order_status = Some((
                format!(
                    "No mid price for {coin} (tried {})",
                    self.mid_candidates_for_symbol(coin).join(", ")
                ),
                true,
            ));
            return Task::none();
        };

        let price = if use_market {
            let coin_is_spot = self.is_spot_coin(coin);
            wire_market_price(
                mid,
                is_buy,
                self.market_slippage_fraction(),
                sz_decimals,
                coin_is_spot,
            )
        } else {
            let coin_is_spot = self.is_spot_coin(coin);
            wire_rounded_price(mid, sz_decimals, coin_is_spot)
        };

        let pct_label = format!("{:.0}%", fraction * 100.0);
        let kind_label = if use_market { "market" } else { "limit" };
        self.order_status = Some((
            format!("Closing {pct_label} of {coin} ({kind_label})..."),
            false,
        ));

        Task::perform(
            place_order(key.into(), asset, is_buy, price, size, order_kind, true),
            |r| Message::ClosePositionResult(Box::new(r)),
        )
    }
}
