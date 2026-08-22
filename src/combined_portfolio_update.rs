use crate::account_analytics::fetch_portfolio_history;
use crate::app_state::TradingTerminal;
use crate::combined_portfolio::CombinedPortfolioWallet;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;

use iced::{Point, Size, Task, window};

const COMBINED_PORTFOLIO_MIN_WIDTH: f32 = 820.0;
const COMBINED_PORTFOLIO_MIN_HEIGHT: f32 = 560.0;

impl TradingTerminal {
    pub(crate) fn update_combined_portfolio(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenCombinedPortfolioWindow => self.open_combined_portfolio_window(),
            Message::CombinedPortfolioAddressChanged(value) => {
                self.combined_portfolio.add_address_input = value.into_string();
                Task::none()
            }
            Message::CombinedPortfolioLabelChanged(value) => {
                self.combined_portfolio.add_label_input = value.into_string();
                Task::none()
            }
            Message::CombinedPortfolioAddWallet => self.add_combined_portfolio_wallet(),
            Message::CombinedPortfolioRemoveWallet(address) => {
                self.remove_combined_portfolio_wallet(address.into_string())
            }
            Message::CombinedPortfolioRefresh => self.refresh_combined_portfolio(),
            Message::CombinedPortfolioLoaded(address, request_id, result) => self
                .apply_combined_portfolio_result(
                    address.into_string(),
                    request_id,
                    result.into_result(),
                ),
            Message::CombinedPortfolioScopeChanged(scope) => {
                self.combined_portfolio.scope = scope;
                Task::none()
            }
            Message::CombinedPortfolioWindowChanged(window) => {
                self.combined_portfolio.window = window;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn open_combined_portfolio_window(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        if let Some(window_id) = self.combined_portfolio.window_id {
            return Task::batch([
                window::gain_focus(window_id),
                self.refresh_combined_portfolio(),
            ]);
        }

        let settings = window::Settings {
            size: Size::new(
                self.combined_portfolio.width,
                self.combined_portfolio.height,
            ),
            min_size: Some(Size::new(
                COMBINED_PORTFOLIO_MIN_WIDTH,
                COMBINED_PORTFOLIO_MIN_HEIGHT,
            )),
            position: self
                .combined_portfolio
                .x
                .zip(self.combined_portfolio.y)
                .map(|(x, y)| crate::window_chrome::restored_position(Point::new(x, y)))
                .unwrap_or(window::Position::Centered),
            ..crate::window_chrome::settings(
                self.custom_window_chrome_active,
                self.window_background_blur_enabled,
            )
        };
        let (window_id, open_task) = window::open(settings);
        self.combined_portfolio.window_id = Some(window_id);
        self.combined_portfolio.open = true;
        self.persist_config();

        Task::batch([
            open_task.map(Message::WindowOpened),
            self.refresh_combined_portfolio(),
        ])
    }

    fn add_combined_portfolio_wallet(&mut self) -> Task<Message> {
        let Some(address) =
            Self::normalize_wallet_address(&self.combined_portfolio.add_address_input)
        else {
            self.push_toast("Invalid wallet address".to_string(), true);
            return Task::none();
        };
        if self
            .combined_portfolio
            .wallets
            .iter()
            .any(|wallet| wallet.address == address)
        {
            self.push_toast("Wallet is already in Combined Portfolio".to_string(), true);
            return Task::none();
        }

        let label = self.combined_portfolio.add_label_input.trim().to_string();
        self.combined_portfolio
            .wallets
            .push(CombinedPortfolioWallet {
                address: address.clone(),
                label,
                loading: false,
                request_id: 0,
                history: None,
                error: None,
                last_updated_ms: None,
            });
        self.combined_portfolio.add_address_input.clear();
        self.combined_portfolio.add_label_input.clear();
        self.persist_config();
        self.refresh_combined_portfolio_wallet(address)
    }

    fn remove_combined_portfolio_wallet(&mut self, address: String) -> Task<Message> {
        let Some(address) = Self::normalize_wallet_address(&address) else {
            return Task::none();
        };
        let original_len = self.combined_portfolio.wallets.len();
        self.combined_portfolio
            .wallets
            .retain(|wallet| wallet.address != address);
        if self.combined_portfolio.wallets.len() != original_len {
            self.persist_config();
        }
        Task::none()
    }

    pub(crate) fn refresh_combined_portfolio(&mut self) -> Task<Message> {
        let addresses = self
            .combined_portfolio
            .wallets
            .iter()
            .map(|wallet| wallet.address.clone())
            .collect::<Vec<_>>();
        Task::batch(
            addresses
                .into_iter()
                .map(|address| self.refresh_combined_portfolio_wallet(address)),
        )
    }

    fn refresh_combined_portfolio_wallet(&mut self, address: String) -> Task<Message> {
        let Some(request_id) = self.combined_portfolio.begin_wallet_refresh(&address) else {
            return Task::none();
        };
        let requested_address = address.clone();
        Task::perform(fetch_portfolio_history(address), move |result| {
            Message::CombinedPortfolioLoaded(
                requested_address.clone().into(),
                request_id,
                result.into(),
            )
        })
    }

    fn apply_combined_portfolio_result(
        &mut self,
        address: String,
        request_id: u64,
        result: Result<crate::account_analytics::PortfolioHistory, String>,
    ) -> Task<Message> {
        let Some(wallet) = self
            .combined_portfolio
            .wallet_mut_for_result(&address, request_id)
        else {
            return Task::none();
        };

        wallet.loading = false;
        match result {
            Ok(history) => {
                wallet.history = Some(history);
                wallet.error = None;
                wallet.last_updated_ms = Some(Self::now_ms());
            }
            Err(error) => {
                wallet.error = Some(redact_sensitive_response_text(&error));
            }
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account_analytics::{PortfolioBucket, PortfolioHistory};

    const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

    #[test]
    fn stale_wallet_result_does_not_replace_newer_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal
            .combined_portfolio
            .wallets
            .push(CombinedPortfolioWallet {
                address: ADDRESS.to_string(),
                label: String::new(),
                loading: false,
                request_id: 0,
                history: None,
                error: None,
                last_updated_ms: None,
            });
        let _ = terminal.refresh_combined_portfolio_wallet(ADDRESS.to_string());
        let stale_request_id = terminal.combined_portfolio.wallets[0].request_id;
        let _ = terminal.refresh_combined_portfolio_wallet(ADDRESS.to_string());

        let _ = terminal.apply_combined_portfolio_result(
            ADDRESS.to_string(),
            stale_request_id,
            Ok(PortfolioHistory {
                buckets: std::collections::HashMap::from([(
                    "allTime".to_string(),
                    PortfolioBucket::default(),
                )]),
            }),
        );

        assert!(terminal.combined_portfolio.wallets[0].history.is_none());
        assert!(terminal.combined_portfolio.wallets[0].loading);
    }
}
