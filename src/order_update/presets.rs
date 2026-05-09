use crate::app_state::TradingTerminal;
use crate::config::OrderPreset;
use crate::message::Message;
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
        if let Ok(v) = self.preset_edit_buffer.parse::<f64>() {
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
                    OrderKind::Limit => update_preset(&mut self.order_presets.limit_usd),
                    OrderKind::Chase => update_preset(&mut self.order_presets.chase_usd),
                }
            } else {
                match kind {
                    OrderKind::Market => update_preset(&mut self.order_presets.market_coin),
                    OrderKind::Limit => update_preset(&mut self.order_presets.limit_coin),
                    OrderKind::Chase => update_preset(&mut self.order_presets.chase_coin),
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
            if !self.preset_is_usd
                && let Err(e) = self.validate_outcome_contract_size(preset.size)
            {
                self.order_status = Some((e, true));
            } else {
                self.outcome_read_only_status("trading");
            }
            self.presets_menu_expanded = false;
            return Task::none();
        }

        let Some(mid) = self
            .resolve_mid_for_symbol(&self.active_symbol)
            .filter(|mid| mid.is_finite() && *mid > 0.0)
        else {
            self.order_status =
                Some(("No mid price available for preset calculation".into(), true));
            return Task::none();
        };

        if preset.size.is_finite() && preset.size > 0.0 {
            let qty = if self.preset_is_usd {
                preset.size / mid
            } else {
                preset.size
            };
            self.order_quantity = format!("{qty:.6}");

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
                if is_buy {
                    Task::perform(async move { Message::PlaceBuy }, |m| m)
                } else {
                    Task::perform(async move { Message::PlaceSell }, |m| m)
                }
            } else if kind == OrderKind::Chase {
                self.presets_menu_expanded = false;
                Task::perform(async move { Message::StartChase(is_buy) }, |m| m)
            } else {
                Task::none()
            }
        } else {
            self.order_status = Some(("Preset size must be a positive finite value".into(), true));
            Task::none()
        }
    }
}
