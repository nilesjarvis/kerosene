use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::{helpers, ws};
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_liquidation_feed(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WsHydromancerLiquidation(msg) => match msg {
                ws::HydromancerWsMessage::Connecting => {
                    self.liquidations_status = "Connecting".to_string();
                }
                ws::HydromancerWsMessage::Resuming => {
                    self.liquidations_status = "Resuming session".to_string();
                }
                ws::HydromancerWsMessage::Connected => {
                    self.liquidations_last_rx_ms = Some(Self::now_ms());
                    self.liquidations_status = "Connected".to_string();
                }
                ws::HydromancerWsMessage::Reconnected => {
                    self.liquidations_last_rx_ms = Some(Self::now_ms());
                    self.liquidations_status = "Reconnected".to_string();
                }
                ws::HydromancerWsMessage::Heartbeat => {
                    self.liquidations_last_rx_ms = Some(Self::now_ms());
                }
                ws::HydromancerWsMessage::Reconnecting {
                    error,
                    retry_delay_secs,
                } => {
                    self.liquidations_status =
                        format!("Reconnecting in {retry_delay_secs}s: {error}");
                }
                ws::HydromancerWsMessage::Disconnected(e) => {
                    self.liquidations_last_rx_ms = None;
                    self.liquidations_status = format!("Disconnected: {e}");
                }
                ws::HydromancerWsMessage::Event(liquidation) => {
                    let liquidation = Self::normalize_liquidation_event(liquidation);
                    self.liquidations_last_rx_ms = Some(Self::now_ms());
                    self.liquidations_status = "Connected".to_string();
                    if self.symbol_key_is_hidden(&liquidation.coin) {
                        return Task::none();
                    }
                    let notional = liquidation.size * liquidation.price;
                    if self.liquidation_alerts_enabled
                        && notional >= self.liquidation_alert_threshold
                    {
                        let (icon, position_type) = if liquidation.is_buy {
                            ("💥", "Short")
                        } else {
                            ("🩸", "Long")
                        };

                        let formatted_notional = helpers::format_usd(&format!("{notional:.0}"));
                        let formatted_price = helpers::format_price(liquidation.price);

                        let msg = format!(
                            "{} LIQUIDATED: {} {}\n{} at {}",
                            icon,
                            position_type,
                            liquidation.coin.to_uppercase(),
                            formatted_notional,
                            formatted_price
                        );

                        self.push_toast(msg, liquidation.is_buy);
                    }

                    self.liquidations.push_front(liquidation.clone());
                    if self.liquidations.len() > 10000 {
                        self.liquidations.truncate(10000);
                    }

                    let event_notional = liquidation.size * liquidation.price;
                    let bucket_ms = liquidation.time_ms / 60_000;
                    let entry = self
                        .liquidation_summary_buckets
                        .entry(bucket_ms)
                        .or_insert((0.0, 0.0));
                    if liquidation.is_buy {
                        entry.1 += event_notional;
                    } else {
                        entry.0 += event_notional;
                    }

                    let chart_bucket_sec = liquidation.time_ms / 1000;
                    let chart_entry = self
                        .liquidation_chart_buckets
                        .entry(chart_bucket_sec)
                        .or_insert((0.0, 0.0));
                    if liquidation.is_buy {
                        chart_entry.1 += event_notional;
                    } else {
                        chart_entry.0 += event_notional;
                    }

                    let now_ms = Self::now_ms();
                    let cutoff = (now_ms / 60_000).saturating_sub(1440);
                    self.liquidation_summary_buckets
                        .retain(|&bucket, _| bucket >= cutoff);

                    let chart_cutoff = (now_ms / 1000).saturating_sub(120);
                    self.liquidation_chart_buckets
                        .retain(|&bucket, _| bucket >= chart_cutoff);
                }
                ws::HydromancerWsMessage::TrackedTrade(_) => {}
            },
            Message::ClearLiquidations => {
                self.liquidations.clear();
                self.liquidation_summary_buckets.clear();
                self.liquidation_chart_buckets.clear();
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::TradingTerminal;
    use crate::message::Message;
    use crate::ws::LiquidationEvent;

    #[test]
    fn clear_liquidations_resets_rows_summary_and_chart_buckets() {
        let mut terminal = TradingTerminal::boot().0;
        let liquidation = LiquidationEvent {
            coin: "HYPE".to_string(),
            price: 25.0,
            size: 4.0,
            is_buy: false,
            time_ms: TradingTerminal::now_ms(),
            method: "market".to_string(),
            liquidated_user: "0x0000000000000000000000000000000000000001".to_string(),
            tx_index: 1,
        };

        let _ = terminal.update_liquidation_feed(Message::WsHydromancerLiquidation(
            crate::ws::HydromancerWsMessage::Event(liquidation),
        ));
        assert!(!terminal.liquidations.is_empty());
        assert!(!terminal.liquidation_summary_buckets.is_empty());
        assert!(!terminal.liquidation_chart_buckets.is_empty());

        let _ = terminal.update_liquidation_feed(Message::ClearLiquidations);

        assert!(terminal.liquidations.is_empty());
        assert!(terminal.liquidation_summary_buckets.is_empty());
        assert!(terminal.liquidation_chart_buckets.is_empty());
    }
}
