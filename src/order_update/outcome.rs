use crate::app_state::TradingTerminal;
use crate::config::OrderPreset;
use crate::helpers::{format_price, parse_number};
use crate::message::Message;
use crate::signing::OrderKind;

use iced::Task;

impl TradingTerminal {
    pub(crate) fn handle_execute_outcome_preset(
        &mut self,
        kind: OrderKind,
        preset: OrderPreset,
        is_buy: bool,
    ) -> Task<Message> {
        if matches!(kind, OrderKind::Chase | OrderKind::Twap) {
            self.order_status = Some(("Outcome automation is not supported yet".into(), true));
            self.presets_menu_expanded = false;
            return Task::none();
        }

        let Some(mid) = self
            .resolve_mid_for_symbol(&self.active_symbol)
            .filter(|mid| mid.is_finite() && *mid > 0.0)
        else {
            self.order_status = Some((
                "No mid price available for outcome preset calculation".into(),
                true,
            ));
            self.presets_menu_expanded = false;
            return Task::none();
        };

        let raw_contracts = if self.preset_is_usd {
            preset.size / mid
        } else {
            preset.size
        };
        let contracts = raw_contracts.floor();
        if !contracts.is_finite() || contracts <= 0.0 {
            self.order_status = Some((
                "Outcome preset resolves to less than one contract".into(),
                true,
            ));
            self.presets_menu_expanded = false;
            return Task::none();
        }

        self.order_kind = kind;
        self.order_quantity_is_usd = false;
        self.order_quantity = format!("{contracts:.0}");
        self.order_percentage = 0.0;

        match kind {
            OrderKind::Limit | OrderKind::LimitIoc => {
                let target_price = if let Some(pct) = preset.price_offset_pct {
                    let offset = pct / 100.0;
                    if is_buy {
                        mid * (1.0 - offset)
                    } else {
                        mid * (1.0 + offset)
                    }
                } else {
                    mid
                };
                let target_price = Self::clamp_outcome_market_price(target_price);
                self.order_price = format_price(target_price);
            }
            OrderKind::Market => self.order_price.clear(),
            OrderKind::Chase | OrderKind::Twap => unreachable!("advanced modes returned early"),
        }

        self.presets_menu_expanded = false;
        if is_buy {
            Task::perform(async { Message::PlaceBuy }, |m| m)
        } else {
            Task::perform(async { Message::PlaceSell }, |m| m)
        }
    }

    pub(crate) fn handle_prefill_outcome_sell(&mut self, balance_coin: String) -> Task<Message> {
        let Some(trade_coin) = self.outcome_trade_coin_for_balance_coin(&balance_coin) else {
            self.order_status = Some((format!("{balance_coin} is not a tradable outcome"), true));
            return Task::none();
        };
        if self.symbol_key_is_hidden(&trade_coin) {
            self.order_status = Some(("Outcome ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }

        let Some(contracts) = self
            .account_data
            .as_ref()
            .and_then(|data| data.spot.balances.iter().find(|b| b.coin == balance_coin))
            .and_then(outcome_available_contracts)
        else {
            self.order_status = Some(("No available outcome contracts to sell".into(), true));
            return Task::none();
        };

        let mut switch_task = Task::none();
        if self.active_symbol != trade_coin {
            switch_task = self.switch_active_symbol_internal(trade_coin.clone());
            if self.active_symbol != trade_coin {
                return switch_task;
            }
        }

        self.order_kind = OrderKind::Limit;
        self.order_quantity_is_usd = false;
        self.order_quantity = format!("{contracts:.0}");
        self.order_percentage = 0.0;
        if let Some(mid) = self.resolve_mid_for_symbol(&trade_coin) {
            self.order_price = format_price(mid);
        } else {
            self.order_price.clear();
        }
        self.presets_menu_expanded = false;
        self.order_status = Some((
            format!(
                "Prepared sell ticket for {:.0} {}",
                contracts, self.active_symbol_display
            ),
            false,
        ));
        self.persist_config();
        switch_task
    }
}

fn outcome_available_contracts(balance: &crate::account::SpotBalance) -> Option<f64> {
    let total = parse_number(&balance.total)?;
    let hold = parse_number(&balance.hold)?;
    let available = (total - hold).floor();
    (available.is_finite() && available > 0.0).then_some(available)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::SpotBalance;

    fn balance(total: &str, hold: &str) -> SpotBalance {
        SpotBalance {
            coin: "+650".to_string(),
            token: None,
            total: total.to_string(),
            hold: hold.to_string(),
            entry_ntl: "0".to_string(),
            supplied: None,
        }
    }

    #[test]
    fn available_outcome_contracts_floor_available_balance() {
        assert_eq!(
            outcome_available_contracts(&balance("10.9", "0.2")),
            Some(10.0)
        );
        assert_eq!(outcome_available_contracts(&balance("1.9", "1.0")), None);
        assert_eq!(outcome_available_contracts(&balance("bad", "0")), None);
    }
}
