use crate::account::AccountData;
use crate::account_analytics::{fetch_income_data, fetch_portfolio_history};
use crate::account_metrics::format_signed_usd_value;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use chrono::{DateTime, Utc};
use iced::Task;

impl TradingTerminal {
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
                if let Some(addr) = &self.connected_address {
                    let requested_addr = addr.clone();
                    self.portfolio.loading = true;
                    return Task::perform(fetch_portfolio_history(addr.clone()), move |r| {
                        Message::PortfolioLoaded(requested_addr.clone(), Box::new(r))
                    });
                }
            }
            Message::PortfolioLoaded(address, result) => {
                if self.connected_address.as_deref() != Some(address.as_str()) {
                    return Task::none();
                }
                self.portfolio.loading = false;
                match *result {
                    Ok(data) => {
                        self.portfolio.data = Some(data);
                        self.portfolio.last_error = None;
                    }
                    Err(e) => {
                        self.portfolio.last_error = Some(e);
                    }
                }
            }
            Message::RefreshIncome => {
                let is_pm = self
                    .account_data
                    .as_ref()
                    .is_some_and(AccountData::is_portfolio_margin);
                if is_pm && let Some(addr) = &self.connected_address {
                    let requested_addr = addr.clone();
                    self.income.loading = true;
                    return Task::perform(fetch_income_data(addr.clone()), move |r| {
                        Message::IncomeLoaded(requested_addr.clone(), Box::new(r))
                    });
                }
            }
            Message::IncomeLoaded(address, result) => {
                if self.connected_address.as_deref() != Some(address.as_str()) {
                    return Task::none();
                }
                self.income.loading = false;
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
                                    let message = format!(
                                        "Hourly interest received: {} ({token_label}, {time_label} UTC)",
                                        format_signed_usd_value(total_positive)
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
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio_state::PnlValueDisplayMode;

    #[test]
    fn portfolio_pnl_value_mode_updates_even_when_pnl_is_hidden() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.hide_pnl = true;
        terminal.portfolio.pnl_value_display_mode = PnlValueDisplayMode::Usd;

        let _ = terminal.update_portfolio_income(Message::SetPortfolioPnlValueMode(
            PnlValueDisplayMode::Percent,
        ));
        assert_eq!(
            terminal.portfolio.pnl_value_display_mode,
            PnlValueDisplayMode::Percent
        );

        let _ = terminal
            .update_portfolio_income(Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Usd));
        assert_eq!(
            terminal.portfolio.pnl_value_display_mode,
            PnlValueDisplayMode::Usd
        );
    }
}
