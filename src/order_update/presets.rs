use crate::app_state::TradingTerminal;
use crate::config::OrderPreset;
use crate::helpers::{parse_number, positive_finite_value};
use crate::message::Message;
use crate::order_execution::OrderSurface;
use crate::signing::OrderKind;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn handle_toggle_presets_menu(&mut self) {
        self.presets_menu_expanded = !self.presets_menu_expanded;
    }

    pub(crate) fn handle_toggle_preset_currency(&mut self) {
        self.preset_is_usd = !self.preset_is_usd;
        self.persist_config();
    }

    pub(crate) fn handle_toggle_preset_edit_mode(&mut self) {
        self.preset_edit_mode = !self.preset_edit_mode;
        self.preset_edit_idx = None;
    }

    pub(crate) fn handle_edit_preset_start(
        &mut self,
        kind: OrderKind,
        idx: usize,
        current_size_str: String,
    ) {
        self.preset_edit_idx = Some((kind, idx));
        self.preset_edit_buffer = current_size_str;
    }

    pub(crate) fn handle_edit_preset_changed(&mut self, new_text: String) {
        self.preset_edit_buffer = new_text;
    }

    pub(crate) fn handle_edit_preset_save(&mut self, kind: OrderKind, idx: usize) {
        if let Some(v) = parse_number(&self.preset_edit_buffer) {
            let prefix = if self.preset_is_usd { "$" } else { "" };
            let suffix = "";

            let update_preset = |presets: &mut Vec<OrderPreset>| {
                if let Some(preset) = presets.get_mut(idx) {
                    preset.size = v;
                    if let Some(pct) = preset.price_offset_pct {
                        preset.label = format!("-{pct}% {prefix}{v}{suffix}");
                    } else {
                        preset.label = format!("{prefix}{v}{suffix}");
                    }
                }
            };

            if self.preset_is_usd {
                match kind {
                    OrderKind::Market => update_preset(&mut self.order_presets.market_usd),
                    OrderKind::Limit | OrderKind::LimitIoc => {
                        update_preset(&mut self.order_presets.limit_usd)
                    }
                    OrderKind::Chase => update_preset(&mut self.order_presets.chase_usd),
                    OrderKind::Twap => {}
                }
            } else {
                match kind {
                    OrderKind::Market => update_preset(&mut self.order_presets.market_coin),
                    OrderKind::Limit | OrderKind::LimitIoc => {
                        update_preset(&mut self.order_presets.limit_coin)
                    }
                    OrderKind::Chase => update_preset(&mut self.order_presets.chase_coin),
                    OrderKind::Twap => {}
                }
            }
            self.persist_config();
        }
        self.preset_edit_idx = None;
    }

    pub(crate) fn handle_execute_preset(
        &mut self,
        kind: OrderKind,
        preset: OrderPreset,
        is_buy: bool,
    ) -> Task<Message> {
        self.order_kind = kind;
        if self.is_outcome_coin(&self.active_symbol) {
            return self.handle_execute_outcome_preset(kind, preset, is_buy);
        }

        let Some(mid) = self
            .resolve_mid_for_symbol(&self.active_symbol)
            .and_then(positive_finite_value)
        else {
            self.order_status =
                Some(("No mid price available for preset calculation".into(), true));
            return Task::none();
        };

        if let Some(preset_size) = positive_finite_value(preset.size) {
            let qty = if self.preset_is_usd {
                preset_size / mid
            } else {
                preset_size
            };
            self.order_quantity = format!("{qty:.6}");
            self.order_quantity_is_usd = false;
            self.order_percentage = 0.0;

            if kind == OrderKind::Limit || kind == OrderKind::Market {
                if kind == OrderKind::Limit {
                    if let Some(pct) = preset.price_offset_pct {
                        let offset = pct / 100.0;
                        let target_price = if is_buy {
                            mid * (1.0 - offset)
                        } else {
                            mid * (1.0 + offset)
                        };
                        self.order_price = format!("{target_price:.4}");
                    }
                } else if kind == OrderKind::Market {
                    self.order_price.clear();
                }

                self.presets_menu_expanded = false;
                self.execute_order_with_surface(is_buy, OrderSurface::Preset)
            } else if kind == OrderKind::Chase {
                self.presets_menu_expanded = false;
                Task::perform(async move { Message::StartChase(is_buy) }, |m| m)
            } else if kind == OrderKind::Twap {
                self.presets_menu_expanded = false;
                Task::perform(async move { Message::StartTwap(is_buy) }, |m| m)
            } else {
                Task::none()
            }
        } else {
            self.order_status = Some(("Preset size must be a positive finite value".into(), true));
            Task::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OrderPreset;
    use crate::signing::OrderKind;

    #[test]
    fn usd_preset_writes_coin_quantity_and_clears_ticket_usd_denomination() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.active_symbol = "BTC".to_string();
        terminal.all_mids.insert("BTC".to_string(), 50_000.0);
        terminal
            .all_mids_updated_at_ms
            .insert("BTC".to_string(), TradingTerminal::now_ms());
        terminal.preset_is_usd = true;
        terminal.order_quantity_is_usd = true;
        terminal.order_percentage = 25.0;

        let _task = terminal.handle_execute_preset(
            OrderKind::Market,
            OrderPreset {
                label: "$100".to_string(),
                size: 100.0,
                price_offset_pct: None,
            },
            true,
        );

        assert_eq!(terminal.order_quantity, "0.002000");
        assert!(!terminal.order_quantity_is_usd);
        assert_eq!(terminal.order_percentage, 0.0);
    }
}
