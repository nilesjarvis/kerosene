use crate::account_state::BottomTab;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Row, Space, button, column, container, row, rule, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Account Bottom Tabs
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_bottom_tabs(&self, active_tab: BottomTab) -> Element<'_, Message> {
        let position_count = self.open_position_tab_count();
        let open_order_count = self.open_order_tab_count();
        let tabs = Row::new()
            .push(bottom_tab_button(
                "Positions",
                Some(position_count),
                active_tab == BottomTab::Positions,
                Message::SwitchBottomTab(BottomTab::Positions),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Open Orders",
                Some(open_order_count),
                active_tab == BottomTab::OpenOrders,
                Message::SwitchBottomTab(BottomTab::OpenOrders),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Balances",
                None,
                active_tab == BottomTab::Balances,
                Message::SwitchBottomTab(BottomTab::Balances),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Trade History",
                None,
                active_tab == BottomTab::TradeHistory,
                Message::SwitchBottomTab(BottomTab::TradeHistory),
            ))
            .push(bottom_tab_separator())
            .push(bottom_tab_button(
                "Funding",
                None,
                active_tab == BottomTab::FundingHistory,
                Message::SwitchBottomTab(BottomTab::FundingHistory),
            ))
            .push(Space::new().width(Fill))
            .push(bottom_tab_separator())
            .push(bottom_journal_button())
            .width(Fill)
            .spacing(0)
            .align_y(iced::Alignment::Center);

        let tabs = bottom_tab_strip(tabs);

        let body: Element<Message> = match active_tab {
            BottomTab::Positions => self.view_positions(),
            BottomTab::OpenOrders => self.view_open_orders(),
            BottomTab::Balances => self.view_balances(),
            BottomTab::TradeHistory => self.view_trade_history(),
            BottomTab::FundingHistory => self.view_funding_history(),
        };

        let content = column![
            container(tabs).padding(iced::Padding {
                top: 10.0,
                right: 10.0,
                bottom: 0.0,
                left: 10.0
            }),
            container(body)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 10.0,
                    bottom: 10.0,
                    left: 10.0
                })
                .width(Fill)
                .height(Fill)
        ]
        .spacing(6);

        container(content).width(Fill).height(Fill).into()
    }

    fn open_position_tab_count(&self) -> usize {
        self.projected_positions()
            .into_iter()
            .filter(|position| !self.symbol_key_is_hidden(&position.asset_position.position.coin))
            .count()
    }

    fn open_order_tab_count(&self) -> usize {
        self.projected_open_orders()
            .into_iter()
            .filter(|row| !self.symbol_key_is_hidden(&row.order.coin))
            .count()
    }
}

fn bottom_tab_strip<'a>(content: Row<'a, Message>) -> Element<'a, Message> {
    container(column![content, bottom_tab_bottom_separator()].spacing(0))
        .width(Fill)
        .style(|theme: &Theme| {
            let background = Color {
                a: 0.04,
                ..theme.extended_palette().background.weak.color
            };
            container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        })
        .into()
}

fn bottom_tab_button(
    label: &'static str,
    count: Option<usize>,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    let mut content = row![text(label).size(11).center()]
        .spacing(6)
        .align_y(iced::Alignment::Center);
    if let Some(count) = count.filter(|count| *count > 0) {
        content = content.push(bottom_tab_count_badge(count, active));
    }

    button(content)
        .on_press(msg)
        .padding([4, 10])
        .style(move |theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(background.into()),
                text_color: if active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            }
        })
        .into()
}

fn bottom_tab_count_badge(count: usize, active: bool) -> Element<'static, Message> {
    container(text(count.to_string()).size(10).center())
        .padding([1, 5])
        .style(move |theme: &Theme| {
            let text_color = if active {
                theme.palette().primary
            } else {
                theme.extended_palette().background.weak.text
            };
            let background = Color {
                a: if active { 0.16 } else { 0.08 },
                ..text_color
            };
            let border_color = Color {
                a: if active { 0.28 } else { 0.14 },
                ..text_color
            };
            container::Style {
                background: Some(background.into()),
                text_color: Some(text_color),
                border: iced::Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: border_color,
                },
                ..Default::default()
            }
        })
        .into()
}

fn bottom_journal_button() -> Element<'static, Message> {
    button(
        row![text("Journal").size(12), text("\u{2197}").size(12)]
            .spacing(4)
            .align_y(iced::Alignment::Center),
    )
    .on_press(Message::AddTradingJournal)
    .padding([4, 12])
    .style(move |t: &Theme, status| {
        let background = match status {
            button::Status::Hovered => t.extended_palette().background.strong.color,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(background.into()),
            text_color: t.extended_palette().background.weak.text,
            border: iced::Border {
                radius: 0.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        }
    })
    .into()
}

fn bottom_tab_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.12,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(18)
    .width(1)
    .into()
}

fn bottom_tab_bottom_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.12,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{
        AccountData, AccountDataCompleteness, AssetPosition, ClearinghouseState, MarginSummary,
        OpenOrder, Position, PositionLeverage, SpotBalance, SpotClearinghouseState, UserFeeRates,
    };
    use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
    use crate::config::MarketUniverseConfig;

    fn account_data(positions: Vec<AssetPosition>, open_orders: Vec<OpenOrder>) -> AccountData {
        AccountData {
            fetch_scope: Default::default(),
            request_weight_estimate: 0,
            account_abstraction: Default::default(),
            clearinghouse: ClearinghouseState {
                margin_summary: MarginSummary {
                    account_value: "0".to_string(),
                    total_ntl_pos: "0".to_string(),
                    total_margin_used: "0".to_string(),
                },
                cross_margin_summary: None,
                cross_maintenance_margin_used: None,
                withdrawable: "0".to_string(),
                asset_positions: positions,
            },
            clearinghouses_by_dex: std::collections::HashMap::new(),
            spot: SpotClearinghouseState {
                balances: Vec::new(),
                portfolio_margin_enabled: false,
                portfolio_margin_ratio: None,
                token_to_available_after_maintenance: None,
            },
            open_orders,
            fills: Vec::new(),
            funding_history: Vec::new(),
            fee_rates: UserFeeRates::default(),
            completeness: AccountDataCompleteness::default(),
            fetched_at_ms: 1_000,
        }
    }

    fn position(coin: &str) -> AssetPosition {
        AssetPosition {
            position: Position {
                coin: coin.to_string(),
                szi: "1".to_string(),
                entry_px: "100".to_string(),
                position_value: "100".to_string(),
                unrealized_pnl: "0".to_string(),
                liquidation_px: None,
                leverage: PositionLeverage {
                    leverage_type: "cross".to_string(),
                    value: 1,
                },
                margin_used: "0".to_string(),
                cum_funding: None,
            },
            liquidation_px: None,
        }
    }

    fn open_order(coin: &str, oid: u64) -> OpenOrder {
        OpenOrder {
            coin: coin.to_string(),
            side: "B".to_string(),
            limit_px: "100".to_string(),
            sz: "1".to_string(),
            oid,
            timestamp: 1,
            reduce_only: None,
        }
    }

    fn spot_balance(coin: &str, total: &str) -> SpotBalance {
        SpotBalance {
            coin: coin.to_string(),
            token: None,
            total: total.to_string(),
            hold: "0".to_string(),
            entry_ntl: "0".to_string(),
            supplied: None,
        }
    }

    fn outcome_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: "OUT95-YES".to_string(),
            category: "outcome".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 100_000_950,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: true,
            market_type: MarketType::Outcome,
            outcome: Some(OutcomeSymbolInfo {
                outcome_id: 95,
                question_id: None,
                question_name: None,
                question_description: None,
                question_class: None,
                question_underlying: None,
                question_expiry: None,
                question_price_thresholds: Vec::new(),
                question_period: None,
                question_named_outcomes: Vec::new(),
                question_settled_named_outcomes: Vec::new(),
                question_fallback_outcome: None,
                bucket_index: None,
                is_question_fallback: false,
                side_index: 0,
                side_name: "Yes".to_string(),
                outcome_name: "Recurring".to_string(),
                description: String::new(),
                class: None,
                underlying: None,
                expiry: None,
                target_price: None,
                period: None,
                quote_symbol: "USDC".to_string(),
                quote_token_index: Some(crate::api::USDC_TOKEN_INDEX),
                encoding: 950,
            }),
        }
    }

    #[test]
    fn bottom_tab_counts_reflect_open_positions_and_orders() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.account_data = Some(account_data(
            vec![position("BTC"), position("ETH")],
            vec![open_order("BTC", 1), open_order("ETH", 2)],
        ));

        assert_eq!(terminal.open_position_tab_count(), 2);
        assert_eq!(terminal.open_order_tab_count(), 2);
    }

    #[test]
    fn bottom_tab_counts_follow_market_universe_visibility() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.market_universe = MarketUniverseConfig::hip3_dex("xyz");
        terminal.account_data = Some(account_data(
            vec![position("xyz:NVDA"), position("flx:NVDA")],
            vec![open_order("xyz:NVDA", 1), open_order("flx:NVDA", 2)],
        ));

        assert_eq!(terminal.open_position_tab_count(), 1);
        assert_eq!(terminal.open_order_tab_count(), 1);
    }

    #[test]
    fn bottom_tab_counts_include_outcome_spot_positions() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#950")];
        let mut data = account_data(Vec::new(), Vec::new());
        data.spot.balances = vec![spot_balance("+950", "30")];
        terminal.account_data = Some(data);

        assert_eq!(terminal.open_position_tab_count(), 1);
    }
}
