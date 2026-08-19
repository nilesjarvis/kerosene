use crate::app_state::TradingTerminal;
use crate::combined_portfolio::{
    CombinedPortfolioAggregate, CombinedPortfolioWallet, aggregate_portfolios,
    latest_account_value, wallet_period_pnl,
};
use crate::journal_views::style::{
    journal_card_style, journal_chip_style, journal_dim, journal_ghost_button_style,
    journal_hairline, journal_muted, journal_primary_button_style, journal_rule_style,
    journal_segment_style, journal_text_input_style, journal_window_style,
};
use crate::message::Message;
use crate::portfolio_state::{
    PORTFOLIO_WINDOWS, PnlValueDisplayMode, PortfolioPnlChart, PortfolioScope,
};

use iced::widget::container as container_style;
use iced::widget::{
    Column, Space, button, canvas, column, container, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Fill, Length, Theme};

const TITLE_BAR_HEIGHT: f32 = 42.0;
const TOOLBAR_HEIGHT: f32 = 56.0;
const KPI_STRIP_HEIGHT: f32 = 70.0;
const WALLET_LIST_WIDTH: f32 = 360.0;
const CHART_HEIGHT: f32 = 230.0;

// ---------------------------------------------------------------------------
// Combined portfolio window
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_combined_portfolio(&self) -> Element<'_, Message> {
        let aggregate = self.combined_portfolio_aggregate();

        let body = row![
            container(self.view_combined_portfolio_wallet_list())
                .width(Length::Fixed(WALLET_LIST_WIDTH))
                .height(Fill),
            rule::vertical(1).style(journal_rule_style),
            container(self.view_combined_portfolio_overview(&aggregate))
                .width(Fill)
                .height(Fill),
        ]
        .width(Fill)
        .height(Fill);

        let mut content: Column<'_, Message> = column![
            self.view_combined_portfolio_title_bar(&aggregate),
            rule::horizontal(1).style(journal_rule_style),
            self.view_combined_portfolio_toolbar(),
            rule::horizontal(1).style(journal_rule_style),
            self.view_combined_portfolio_kpis(&aggregate),
            rule::horizontal(1).style(journal_rule_style),
        ]
        .width(Fill)
        .height(Fill);

        let failed = self
            .combined_portfolio
            .wallets
            .iter()
            .filter(|wallet| wallet.error.is_some())
            .count();
        if failed > 0 {
            let noun = if failed == 1 { "wallet" } else { "wallets" };
            content = content
                .push(combined_portfolio_warning_bar(format!(
                    "{failed} {noun} could not be refreshed. Showing the remaining portfolio data."
                )))
                .push(rule::horizontal(1).style(journal_rule_style));
        }

        container(content.push(body))
            .width(Fill)
            .height(Fill)
            .style(journal_window_style)
            .into()
    }

    fn combined_portfolio_aggregate(&self) -> CombinedPortfolioAggregate {
        let histories = self
            .combined_portfolio
            .wallets
            .iter()
            .filter_map(|wallet| wallet.history.as_ref())
            .collect::<Vec<_>>();
        aggregate_portfolios(
            &histories,
            self.combined_portfolio.scope,
            self.combined_portfolio.window,
            self.status_bar_now_ms,
        )
    }

    fn view_combined_portfolio_title_bar(
        &self,
        aggregate: &CombinedPortfolioAggregate,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let total_wallets = self.combined_portfolio.wallets.len();
        let wallet_label = if total_wallets == 1 {
            "1 wallet".to_string()
        } else {
            format!("{total_wallets} wallets")
        };

        let badge = container(
            text("WATCH ONLY")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
        )
        .padding([2, 8])
        .style(journal_chip_style(theme.palette().primary));

        let status = if aggregate.loaded_wallets == total_wallets && total_wallets > 0 {
            "All wallets loaded".to_string()
        } else {
            format!("{} of {total_wallets} loaded", aggregate.loaded_wallets)
        };

        container(
            row![
                text("Combined Portfolio")
                    .size(19)
                    .color(theme.palette().text),
                badge,
                text(wallet_label)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_dim(&theme)),
                Space::new().width(Fill),
                text(status)
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(&theme)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding([0, 16])
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn view_combined_portfolio_toolbar(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let loading = self
            .combined_portfolio
            .wallets
            .iter()
            .any(|wallet| wallet.loading);

        let address = text_input(
            "0x wallet address",
            &self.combined_portfolio.add_address_input,
        )
        .on_input(|value| Message::CombinedPortfolioAddressChanged(value.into()))
        .on_submit(Message::CombinedPortfolioAddWallet)
        .padding([7, 10])
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .style(journal_text_input_style)
        .width(Length::Fixed(260.0));
        let label = text_input("Label (optional)", &self.combined_portfolio.add_label_input)
            .on_input(|value| Message::CombinedPortfolioLabelChanged(value.into()))
            .on_submit(Message::CombinedPortfolioAddWallet)
            .padding([7, 10])
            .size(11)
            .style(journal_text_input_style)
            .width(Length::Fixed(150.0));
        let add = button(
            text("Add wallet")
                .size(11)
                .font(crate::app_fonts::monospace_font()),
        )
        .on_press(Message::CombinedPortfolioAddWallet)
        .padding([7, 12])
        .style(journal_primary_button_style);
        let refresh = button(
            text(if loading {
                "Refreshing…"
            } else {
                "Refresh all"
            })
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        )
        .on_press(Message::CombinedPortfolioRefresh)
        .padding([6, 10])
        .style(journal_ghost_button_style);

        let scope = row![
            toolbar_caption("SCOPE", &theme),
            segment_button(
                "All",
                self.combined_portfolio.scope == PortfolioScope::All,
                Message::CombinedPortfolioScopeChanged(PortfolioScope::All),
            ),
            segment_button(
                "Perp",
                self.combined_portfolio.scope == PortfolioScope::Perp,
                Message::CombinedPortfolioScopeChanged(PortfolioScope::Perp),
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        container(
            row![
                address,
                label,
                add,
                Space::new().width(Fill),
                scope,
                refresh
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .height(Length::Fixed(TOOLBAR_HEIGHT))
        .padding([0, 16])
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn view_combined_portfolio_kpis(
        &self,
        aggregate: &CombinedPortfolioAggregate,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let total_pnl = aggregate
            .total_pnl
            .map(|value| denomination.format_signed_value(value, 2))
            .unwrap_or_else(|| "—".to_string());
        let account_value = aggregate
            .account_value
            .map(|value| denomination.format_value(value, 2))
            .unwrap_or_else(|| "—".to_string());
        let total_color = aggregate
            .total_pnl
            .map(|value| crate::helpers::signed_number_color(value, &theme))
            .unwrap_or(theme.palette().text);
        let loading = self
            .combined_portfolio
            .wallets
            .iter()
            .filter(|wallet| wallet.loading)
            .count();
        let status = if loading > 0 {
            format!("{loading} syncing")
        } else if aggregate.loaded_wallets > 0 {
            "Current".to_string()
        } else {
            "Not loaded".to_string()
        };

        let cells = [
            kpi_cell("TOTAL PNL", total_pnl, total_color, &theme),
            kpi_cell(
                "COMBINED VALUE",
                account_value,
                theme.palette().text,
                &theme,
            ),
            kpi_cell(
                "WALLETS",
                self.combined_portfolio.wallets.len().to_string(),
                theme.palette().text,
                &theme,
            ),
            kpi_cell(
                "PROFITABLE",
                aggregate.profitable_wallets.to_string(),
                theme.palette().success,
                &theme,
            ),
            kpi_cell(
                "STATUS",
                status,
                if loading > 0 {
                    theme.palette().primary
                } else {
                    journal_muted(&theme)
                },
                &theme,
            ),
        ];

        let strip = cells.into_iter().enumerate().fold(
            row![].height(Fill).align_y(Alignment::Center),
            |strip, (index, cell)| {
                if index == 0 {
                    strip.push(cell)
                } else {
                    strip
                        .push(rule::vertical(1).style(journal_rule_style))
                        .push(cell)
                }
            },
        );

        container(strip)
            .width(Fill)
            .height(Length::Fixed(KPI_STRIP_HEIGHT))
            .into()
    }

    fn view_combined_portfolio_wallet_list(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let count = self.combined_portfolio.wallets.len();
        let heading = row![
            text("WALLETS")
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(&theme)),
            Space::new().width(Fill),
            text(count.to_string())
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(&theme)),
        ];

        let list = if self.combined_portfolio.wallets.is_empty() {
            column![
                container(
                    column![
                        text("No wallets yet").size(13).color(theme.palette().text),
                        text("Add any Hyperliquid wallet above to include its portfolio history.")
                            .size(11)
                            .color(journal_muted(&theme)),
                    ]
                    .spacing(6),
                )
                .width(Fill)
                .padding(14)
                .style(journal_card_style),
            ]
        } else {
            self.combined_portfolio.wallets.iter().enumerate().fold(
                column![].spacing(8),
                |list, (index, wallet)| {
                    list.push(self.view_combined_portfolio_wallet_card(index, wallet))
                },
            )
        };

        let scroll = scrollable(list)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4.0)
                    .scroller_width(4.0),
            ))
            .height(Fill);

        container(column![heading, scroll].spacing(10))
            .width(Fill)
            .height(Fill)
            .padding(14)
            .into()
    }

    fn view_combined_portfolio_wallet_card(
        &self,
        index: usize,
        wallet: &CombinedPortfolioWallet,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let primary = if wallet.label.trim().is_empty() {
            self.wallet_label(&wallet.address)
                .map(str::to_string)
                .unwrap_or_else(|| format!("Wallet {}", index + 1))
        } else {
            wallet.label.clone()
        };
        let address = Self::short_address(&wallet.address);
        let pnl = wallet.history.as_ref().and_then(|history| {
            wallet_period_pnl(
                history,
                self.combined_portfolio.scope,
                self.combined_portfolio.window,
                self.status_bar_now_ms,
            )
        });
        let value = wallet
            .history
            .as_ref()
            .and_then(|history| latest_account_value(history, self.combined_portfolio.scope));
        let denomination = self.display_denomination_context();
        let pnl_text = pnl
            .map(|value| denomination.format_signed_value(value, 2))
            .unwrap_or_else(|| "—".to_string());
        let value_text = value
            .map(|value| denomination.format_value(value, 2))
            .unwrap_or_else(|| "—".to_string());
        let status = if wallet.loading {
            ("SYNCING", theme.palette().primary)
        } else if wallet.error.is_some() {
            ("STALE", theme.palette().warning)
        } else if wallet.history.is_some() {
            ("READY", theme.palette().success)
        } else {
            ("PENDING", journal_muted(&theme))
        };

        let header = row![
            column![
                text(primary).size(13).color(theme.palette().text),
                text(address)
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_dim(&theme)),
            ]
            .spacing(2),
            Space::new().width(Fill),
            container(
                text(status.0)
                    .size(9)
                    .font(crate::app_fonts::monospace_font()),
            )
            .padding([2, 6])
            .style(journal_chip_style(status.1)),
        ]
        .align_y(Alignment::Center);

        let metrics = row![
            wallet_metric("PERIOD PNL", pnl_text, pnl, &theme),
            rule::vertical(1).style(journal_rule_style),
            wallet_metric("VALUE", value_text, None, &theme),
        ]
        .height(42)
        .align_y(Alignment::Center);

        let actions = row![
            button(
                text("Details")
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
            )
            .on_press(Message::OpenWalletDetailsWindow(
                wallet.address.clone().into(),
            ))
            .padding([4, 8])
            .style(journal_ghost_button_style),
            Space::new().width(Fill),
            button(
                text("Remove")
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
            )
            .on_press(Message::CombinedPortfolioRemoveWallet(
                wallet.address.clone().into(),
            ))
            .padding([4, 8])
            .style(journal_ghost_button_style),
        ]
        .align_y(Alignment::Center);

        let mut content = column![
            header,
            rule::horizontal(1).style(journal_rule_style),
            metrics
        ]
        .spacing(8);
        if let Some(error) = wallet.error.as_deref() {
            content = content.push(
                text(format!("Refresh failed: {error}"))
                    .size(9)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.palette().warning),
            );
        }
        content = content.push(actions);

        container(content)
            .width(Fill)
            .padding(12)
            .style(journal_card_style)
            .into()
    }

    fn view_combined_portfolio_overview(
        &self,
        aggregate: &CombinedPortfolioAggregate,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        if self.combined_portfolio.wallets.is_empty() {
            return center_message(
                "Add wallets to build a combined PnL view.",
                journal_muted(&theme),
            );
        }

        let loading = self
            .combined_portfolio
            .wallets
            .iter()
            .any(|wallet| wallet.loading);
        if aggregate.loaded_wallets == 0 {
            let message = if loading {
                "Loading combined portfolio…"
            } else {
                "No portfolio history is available yet."
            };
            return center_message(message, journal_muted(&theme));
        }

        let denomination = self.display_denomination_context();
        let pnl_text = aggregate
            .total_pnl
            .map(|value| denomination.format_signed_value(value, 2))
            .unwrap_or_else(|| "—".to_string());
        let pnl_color = aggregate
            .total_pnl
            .map(|value| crate::helpers::signed_number_color(value, &theme))
            .unwrap_or(theme.palette().text);
        let scope = match self.combined_portfolio.scope {
            PortfolioScope::All => "ALL PORTFOLIO",
            PortfolioScope::Perp => "PERPETUALS",
        };
        let caption = format!(
            "COMBINED PNL · {scope} · {}",
            self.combined_portfolio.window.label()
        );

        let hero = container(
            column![
                text(caption)
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(&theme)),
                text(pnl_text)
                    .size(34)
                    .font(crate::app_fonts::monospace_font())
                    .color(pnl_color),
                text(format!(
                    "{} loaded · {} profitable",
                    aggregate.loaded_wallets, aggregate.profitable_wallets
                ))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(&theme)),
            ]
            .spacing(5),
        )
        .width(Fill)
        .padding(18)
        .style(journal_card_style);

        let chart: Element<'_, Message> = if aggregate.points.len() >= 2 {
            canvas(PortfolioPnlChart {
                points: aggregate.points.clone(),
                value_mode: PnlValueDisplayMode::Usd,
                denomination,
            })
            .width(Fill)
            .height(CHART_HEIGHT)
            .into()
        } else {
            center_message_with_height(
                "More history is needed to draw the combined chart.",
                journal_muted(&theme),
                CHART_HEIGHT,
            )
        };

        let windows = PORTFOLIO_WINDOWS.iter().fold(
            row![].spacing(4).align_y(Alignment::Center),
            |track, window| {
                track.push(segment_button(
                    window.label(),
                    self.combined_portfolio.window == *window,
                    Message::CombinedPortfolioWindowChanged(*window),
                ))
            },
        );

        let content = column![
            hero,
            column![
                text("AGGREGATE HISTORY")
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(&theme)),
                chart,
                windows,
            ]
            .spacing(10),
        ]
        .spacing(18)
        .width(Fill);

        scrollable(container(content).width(Fill).padding(18))
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4.0)
                    .scroller_width(4.0),
            ))
            .height(Fill)
            .into()
    }
}

fn segment_button(label: &str, active: bool, message: Message) -> Element<'static, Message> {
    button(
        text(label.to_string())
            .size(10)
            .font(crate::app_fonts::monospace_font()),
    )
    .on_press(message)
    .padding([4, 9])
    .style(journal_segment_style(active))
    .into()
}

fn toolbar_caption(label: &str, theme: &Theme) -> Element<'static, Message> {
    text(label.to_string())
        .size(9)
        .font(crate::app_fonts::monospace_font())
        .color(journal_dim(theme))
        .into()
}

fn kpi_cell(label: &str, value: String, color: Color, theme: &Theme) -> Element<'static, Message> {
    container(
        column![
            text(label.to_string())
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(theme)),
            text(value)
                .size(16)
                .font(crate::app_fonts::monospace_font())
                .color(color),
        ]
        .spacing(4),
    )
    .width(Fill)
    .padding([0, 16])
    .into()
}

fn wallet_metric(
    label: &str,
    value: String,
    signed_value: Option<f64>,
    theme: &Theme,
) -> Element<'static, Message> {
    let color = signed_value
        .map(|value| crate::helpers::signed_number_color(value, theme))
        .unwrap_or(theme.palette().text);
    container(
        column![
            text(label.to_string())
                .size(8)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(theme)),
            text(value)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(color),
        ]
        .spacing(3),
    )
    .width(Fill)
    .into()
}

fn combined_portfolio_warning_bar(message: String) -> Element<'static, Message> {
    container(
        text(message)
            .size(11)
            .font(crate::app_fonts::monospace_font()),
    )
    .width(Fill)
    .padding([6, 16])
    .style(|theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.08,
                ..theme.palette().warning
            }
            .into(),
        ),
        text_color: Some(theme.palette().warning),
        border: Border {
            color: journal_hairline(theme),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container_style::Style::default()
    })
    .into()
}

fn center_message(message: &str, color: Color) -> Element<'static, Message> {
    container(
        text(message.to_string())
            .size(13)
            .font(crate::app_fonts::monospace_font())
            .color(color),
    )
    .width(Fill)
    .height(Fill)
    .center(Fill)
    .into()
}

fn center_message_with_height(
    message: &str,
    color: Color,
    height: f32,
) -> Element<'static, Message> {
    container(
        text(message.to_string())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(color),
    )
    .width(Fill)
    .height(Length::Fixed(height))
    .center(Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account_analytics::{PortfolioBucket, PortfolioHistory};
    use crate::combined_portfolio::CombinedPortfolioWallet;
    use crate::portfolio_state::PortfolioWindow;

    #[test]
    fn combined_portfolio_view_builds_empty_loading_and_loaded_states() {
        let (mut terminal, _) = TradingTerminal::boot();
        let _ = terminal.view_combined_portfolio();

        terminal
            .combined_portfolio
            .wallets
            .push(CombinedPortfolioWallet {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                label: "Primary".to_string(),
                loading: true,
                request_id: 1,
                history: None,
                error: None,
                last_updated_ms: None,
            });
        let _ = terminal.view_combined_portfolio();

        terminal.combined_portfolio.wallets[0].loading = false;
        terminal.combined_portfolio.wallets[0].history = Some(PortfolioHistory {
            buckets: std::collections::HashMap::from([(
                "allTime".to_string(),
                PortfolioBucket {
                    pnl_history: vec![(1, 0.0), (2, 25.0)],
                    account_value_history: vec![(1, 1_000.0), (2, 1_025.0)],
                    ..PortfolioBucket::default()
                },
            )]),
        });
        let _ = terminal.view_combined_portfolio();
    }

    #[test]
    fn combined_portfolio_cutoff_uses_the_update_driven_reference_time() {
        const DAY_MS: u64 = 24 * 60 * 60 * 1000;
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.combined_portfolio.window = PortfolioWindow::Week;
        terminal.status_bar_now_ms = 20 * DAY_MS;
        terminal
            .combined_portfolio
            .wallets
            .push(CombinedPortfolioWallet {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                label: "Primary".to_string(),
                loading: false,
                request_id: 1,
                history: Some(PortfolioHistory {
                    buckets: std::collections::HashMap::from([(
                        "allTime".to_string(),
                        PortfolioBucket {
                            pnl_history: vec![
                                (10 * DAY_MS, 20.0),
                                (14 * DAY_MS, 50.0),
                                (18 * DAY_MS, 80.0),
                                (20 * DAY_MS, 100.0),
                            ],
                            account_value_history: vec![(20 * DAY_MS, 1_100.0)],
                            ..PortfolioBucket::default()
                        },
                    )]),
                }),
                error: None,
                last_updated_ms: None,
            });

        assert_eq!(
            terminal.combined_portfolio_aggregate().total_pnl,
            Some(80.0)
        );

        terminal.status_bar_now_ms = 21 * DAY_MS;
        assert_eq!(
            terminal.combined_portfolio_aggregate().total_pnl,
            Some(50.0)
        );
    }
}
