use crate::account_analytics::{fetch_income_data, fetch_portfolio_history};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use chrono::{DateTime, Utc};
use iced::Task;

impl TradingTerminal {
    pub(crate) fn start_portfolio_refresh_for_address(&mut self, address: String) -> Task<Message> {
        if self.portfolio.loading {
            self.portfolio.queue_refresh_followup();
            return Task::none();
        }
        let requested_addr = address.clone();
        let request_id = self.portfolio.begin_refresh();
        Task::perform(fetch_portfolio_history(address), move |r| {
            Message::PortfolioLoaded(requested_addr.clone(), request_id, Box::new(r))
        })
    }

    pub(crate) fn start_income_refresh_for_address(&mut self, address: String) -> Task<Message> {
        if self.income.loading {
            self.income.queue_refresh_followup();
            return Task::none();
        }
        let requested_addr = address.clone();
        let request_id = self.income.begin_refresh();
        Task::perform(fetch_income_data(address), move |r| {
            Message::IncomeLoaded(requested_addr.clone(), request_id, Box::new(r))
        })
    }

    pub(crate) fn update_portfolio_income(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetPortfolioScope(scope) if self.portfolio.scope != scope => {
                self.portfolio.scope = scope;
            }
            Message::SetPortfolioWindow(window) if self.portfolio.window != window => {
                self.portfolio.window = window;
            }
            Message::SetPortfolioPnlValueMode(mode)
                if self.portfolio.pnl_value_display_mode != mode =>
            {
                self.portfolio.pnl_value_display_mode = mode;
            }
            Message::RefreshPortfolio => {
                if let Some(addr) = self.connected_address.clone() {
                    return self.start_portfolio_refresh_for_address(addr);
                }
            }
            Message::PortfolioLoaded(address, request_id, result) => {
                if !self.portfolio.finish_refresh(request_id) {
                    return Task::none();
                }
                let followup_pending = self.portfolio.take_refresh_followup();
                if self.connected_address.as_deref() != Some(address.as_str()) {
                    if followup_pending && let Some(addr) = self.connected_address.clone() {
                        return self.start_portfolio_refresh_for_address(addr);
                    }
                    return Task::none();
                }
                match *result {
                    Ok(data) => {
                        self.portfolio.data = Some(data);
                        self.portfolio.last_error = None;
                    }
                    Err(e) => {
                        self.portfolio.last_error = Some(e);
                    }
                }
                if followup_pending && let Some(addr) = self.connected_address.clone() {
                    return self.start_portfolio_refresh_for_address(addr);
                }
            }
            Message::RefreshIncome => {
                let is_pm = self
                    .connected_order_account_snapshot()
                    .is_some_and(|(_, data)| data.is_portfolio_margin());
                if is_pm && let Some(addr) = self.connected_address.clone() {
                    return self.start_income_refresh_for_address(addr);
                }
            }
            Message::IncomeLoaded(address, request_id, result) => {
                if !self.income.finish_refresh(request_id) {
                    return Task::none();
                }
                let followup_pending = self.income.take_refresh_followup();
                if self.connected_address.as_deref() != Some(address.as_str()) {
                    if followup_pending {
                        let is_pm = self
                            .connected_order_account_snapshot()
                            .is_some_and(|(_, data)| data.is_portfolio_margin());
                        if is_pm && let Some(addr) = self.connected_address.clone() {
                            return self.start_income_refresh_for_address(addr);
                        }
                    }
                    return Task::none();
                }
                match *result {
                    Ok(data) => {
                        let latest_payment =
                            data.recent_hourly_payments.iter().map(|p| p.time).max();
                        if let Some(latest_time) = latest_payment {
                            if let Some(last_seen) = self.last_income_alert_time
                                && self.income_alerts_enabled
                                && latest_time > last_seen
                            {
                                let mut token_count = 0_usize;
                                let mut total_positive = 0.0_f64;
                                for payment in &data.recent_hourly_payments {
                                    if payment.time == latest_time && payment.net > 0.0 {
                                        total_positive += payment.net;
                                        token_count += 1;
                                    }
                                }

                                if total_positive > 0.0 {
                                    let time_label = i64::try_from(latest_time)
                                        .ok()
                                        .and_then(DateTime::<Utc>::from_timestamp_millis)
                                        .map(|dt| dt.format("%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| "latest hour".to_string());
                                    let token_label = if token_count == 1 {
                                        "1 token".to_string()
                                    } else {
                                        format!("{token_count} tokens")
                                    };
                                    let display_value =
                                        self.format_display_signed_usd_value(total_positive);
                                    let message = format!(
                                        "Hourly interest received: {display_value} \
                                        ({token_label}, {time_label} UTC)"
                                    );
                                    self.push_interest_alert(message);
                                }
                            }
                            self.last_income_alert_time = Some(latest_time);
                        }
                        self.income.data = Some(data);
                        self.income.last_error = None;
                    }
                    Err(e) => {
                        self.income.last_error = Some(e);
                    }
                }
                if followup_pending {
                    let is_pm = self
                        .connected_order_account_snapshot()
                        .is_some_and(|(_, data)| data.is_portfolio_margin());
                    if is_pm && let Some(addr) = self.connected_address.clone() {
                        return self.start_income_refresh_for_address(addr);
                    }
                }
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests;
