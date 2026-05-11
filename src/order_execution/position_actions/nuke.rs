use super::super::pricing::wire_market_price;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{OrderKind, float_to_wire, place_order};

use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
struct NukePositionOrder {
    asset: u32,
    is_buy: bool,
    price: String,
    size: String,
}

fn build_nuke_position_order(
    asset: u32,
    sz_decimals: u32,
    mid: f64,
    szi: f64,
    slippage: f64,
) -> Option<NukePositionOrder> {
    if !mid.is_finite()
        || mid <= 0.0
        || !szi.is_finite()
        || szi.abs() <= 1e-12
        || !slippage.is_finite()
        || slippage < 0.0
    {
        return None;
    }

    let is_buy = szi < 0.0;
    Some(NukePositionOrder {
        asset,
        is_buy,
        price: wire_market_price(mid, is_buy, slippage, sz_decimals, false),
        size: float_to_wire(szi.abs()),
    })
}

fn parse_nuke_position_size(coin: &str, raw_size: &str) -> Result<Option<f64>, String> {
    let size = raw_size
        .trim()
        .parse::<f64>()
        .map_err(|e| format!("NUKE aborted: invalid position size for {coin}: {e}"))?;
    if !size.is_finite() {
        return Err(format!("NUKE aborted: non-finite position size for {coin}"));
    }
    if size.abs() <= 1e-12 {
        Ok(None)
    } else {
        Ok(Some(size))
    }
}

impl TradingTerminal {
    pub(crate) fn execute_nuke_positions(&mut self) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        let positions = self
            .account_data
            .as_ref()
            .map(|d| d.clearinghouse.asset_positions.clone())
            .unwrap_or_default();

        let mut active = Vec::new();
        for ap in positions {
            let coin = ap.position.coin.clone();
            let szi = match parse_nuke_position_size(&coin, &ap.position.szi) {
                Ok(Some(szi)) => szi,
                Ok(None) => continue,
                Err(e) => {
                    self.order_status = Some((e, true));
                    return Task::none();
                }
            };
            if !self.is_ticker_muted(&coin) && !self.position_is_hidden(&coin) {
                active.push((ap, szi));
            }
        }

        if active.is_empty() {
            self.order_status = Some(("No positions to close".into(), true));
            return Task::none();
        }
        self.nuke_confirmation = None;

        let mut tasks = Vec::new();

        for (ap, szi) in &active {
            let coin = &ap.position.coin;

            let sym = self.exchange_symbols.iter().find(|s| s.key == *coin);
            let Some(sym) = sym else {
                continue;
            };
            if sym.market_type != MarketType::Perp {
                continue;
            }

            let Some(mid) = self.resolve_mid_for_symbol(coin) else {
                continue;
            };

            let Some(order) = build_nuke_position_order(
                sym.asset_index,
                sym.sz_decimals,
                mid,
                *szi,
                self.market_slippage_fraction(),
            ) else {
                continue;
            };

            let k = key.clone();
            tasks.push(Task::perform(
                place_order(
                    k.into(),
                    order.asset,
                    order.is_buy,
                    order.price,
                    order.size,
                    OrderKind::Market,
                    true,
                ),
                |r| Message::NukeResult(Box::new(r)),
            ));
        }

        if tasks.is_empty() {
            self.order_status = Some(("No valid positions to close".into(), true));
            return Task::none();
        }

        self.order_status = Some((
            format!(
                "Nuking {} position{}...",
                tasks.len(),
                if tasks.len() == 1 { "" } else { "s" }
            ),
            false,
        ));
        Task::batch(tasks)
    }
}
