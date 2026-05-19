use crate::account_metrics;
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers;
use crate::hyperdash_api::{PerpDeltaEntry, PerpDeltas, TickerPositionEntry, TickerPositions};
use crate::message::Message;
use crate::positioning_state::{
    POSITIONING_CHANGE_ROW_LIMIT, POSITIONING_INFO_LIMIT, PositioningInfoChangeSortField,
    PositioningInfoChangeTimeframe, PositioningInfoId, PositioningInfoInstance,
    PositioningInfoPage, PositioningInfoSide, PositioningInfoSortField,
};
use crate::wallet_state::address_book::WalletDisplay;

use iced::alignment::Horizontal;
use iced::widget::{
    Column, Row, Space, button, column, container, row, rule, scrollable, text, tooltip,
};
use iced::widget::{responsive, text_input};
use iced::{Alignment, Color, Element, Fill, Font, Length, Theme, color};

// ---------------------------------------------------------------------------
// Positioning Information View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_positioning_info(&self, id: PositioningInfoId) -> Element<'_, Message> {
        responsive(move |size| self.view_positioning_info_sized(id, size.width)).into()
    }

    fn view_positioning_info_sized(
        &self,
        id: PositioningInfoId,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(instance) = self.positioning_infos.get(&id) else {
            return container(
                text("Positioning Information instance missing")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        };

        let navigation = self.view_positioning_info_navigation(instance);
        let body = match instance.page {
            PositioningInfoPage::Positions => {
                self.view_positioning_info_positions_page(instance, available_width, &theme)
            }
            PositioningInfoPage::Change => {
                self.view_positioning_info_change_page(instance, available_width, &theme)
            }
        };

        container(column![navigation, rule::horizontal(1), body])
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_positioning_info_positions_page<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let search = text_input("Select perp ticker...", &instance.search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::PositioningInfoSearchChanged(instance.id, q))
            .size(12)
            .padding([5, 8]);
        let autocomplete =
            self.view_positioning_info_autocomplete(instance.id, &instance.search_query, theme);
        let controls = self.view_positioning_info_controls(instance);

        let mut content = column![
            self.view_positioning_info_title(instance, theme),
            search,
            autocomplete,
            controls,
        ]
        .spacing(8);

        if let Some(error) = &instance.error {
            content = content.push(
                text(error.clone())
                    .size(11)
                    .color(if instance.data.is_some() {
                        theme.palette().warning
                    } else {
                        theme.palette().danger
                    }),
            );
        }

        if let Some(data) = &instance.data {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_summary(data, instance, theme))
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_table(data, instance, available_width, theme));
        } else {
            let status: Element<'_, Message> = if instance.loading {
                row![
                    self.view_spinner(18),
                    text("Loading positioning data...")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else if instance.error.is_none() {
                text("No positioning data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            } else {
                text("Positioning data unavailable")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            };
            content = content
                .push(rule::horizontal(1))
                .push(container(status).width(Fill).height(Fill).center(Fill));
        }

        container(scrollable(content))
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn view_positioning_info_change_page<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let search = text_input("Select perp ticker...", &instance.search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::PositioningInfoSearchChanged(instance.id, q))
            .size(12)
            .padding([5, 8]);
        let autocomplete =
            self.view_positioning_info_autocomplete(instance.id, &instance.search_query, theme);
        let controls = self.view_positioning_info_change_controls(instance);

        let mut content = column![
            self.view_positioning_info_title(instance, theme),
            search,
            autocomplete,
            controls,
        ]
        .spacing(8);

        if let Some(error) = &instance.change_error {
            content = content.push(
                text(error.clone())
                    .size(11)
                    .color(if instance.change_data.is_some() {
                        theme.palette().warning
                    } else {
                        theme.palette().danger
                    }),
            );
        }

        if let Some(data) = &instance.change_data {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_change_summary(data, instance, theme))
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_change_table(
                    data,
                    instance,
                    available_width,
                    theme,
                ));
        } else {
            let status: Element<'_, Message> = if instance.change_loading {
                row![
                    self.view_spinner(18),
                    text("Loading position changes...")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else if instance.change_error.is_none() {
                text("No change data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            } else {
                text("Change data unavailable")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            };
            content = content
                .push(rule::horizontal(1))
                .push(container(status).width(Fill).height(Fill).center(Fill));
        }

        container(scrollable(content))
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn view_positioning_info_navigation(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'static, Message> {
        let nav = PositioningInfoPage::ALL.iter().fold(
            Row::new().spacing(4).align_y(Alignment::Center),
            |row, &page| {
                row.push(positioning_navigation_button(
                    instance.id,
                    page,
                    instance.page == page,
                ))
            },
        );

        container(nav)
            .width(Fill)
            .padding([8, 10])
            .into()
    }

    fn view_positioning_info_title<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let display = self.positioning_info_symbol_display(&instance.symbol);
        let mut symbol_row = Row::new().spacing(6).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(&instance.symbol, 16, theme.palette().text) {
            symbol_row = symbol_row.push(icon);
        }
        symbol_row = symbol_row.push(
            text(format!("Positioning Information ({display})"))
                .size(13)
                .color(theme.palette().text),
        );
        if let Some(dex) = helpers::hip3_dex(&instance.symbol) {
            symbol_row = symbol_row.push(
                text(format!("({dex})"))
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        let status: Element<'_, Message> = if instance.loading {
            row![
                self.view_spinner(14),
                text("Refreshing")
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(5)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(
                instance
                    .last_fetch_ms
                    .map(|last| {
                        format!(
                            "{} ago",
                            helpers::format_relative_time(last, TradingTerminal::now_ms())
                        )
                    })
                    .unwrap_or_else(|| "Not loaded".to_string()),
            )
            .size(10)
            .color(theme.extended_palette().background.weak.text)
            .into()
        };

        row![
            symbol_row.width(Fill),
            status,
            button(text("Refresh").size(10))
                .style(button::text)
                .on_press(Message::RefreshPositioningInfoPane(instance.id))
                .padding([2, 6]),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_positioning_info_autocomplete<'a>(
        &'a self,
        id: PositioningInfoId,
        search_query: &str,
        theme: &Theme,
    ) -> Column<'a, Message> {
        let query = search_query.trim().to_lowercase();
        let mut autocomplete = Column::new().spacing(3);
        if query.is_empty() {
            return autocomplete;
        }

        let mut matches: Vec<&ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Perp)
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| positioning_symbol_matches(symbol, &query))
            .collect();
        matches.sort_by(|a, b| {
            a.ticker
                .cmp(&b.ticker)
                .then_with(|| helpers::compare_symbol_keys_for_same_ticker(&a.key, &b.key))
        });
        matches.truncate(5);

        for symbol in matches {
            let sym_key = symbol.key.clone();
            let display = symbol
                .display_name
                .as_deref()
                .unwrap_or(&symbol.ticker)
                .to_string();
            let mut coin_content = Row::new().spacing(6).align_y(Alignment::Center);
            if let Some(icon) = helpers::symbol_icon(&sym_key, 14, theme.palette().text) {
                coin_content = coin_content.push(icon);
            }
            coin_content = coin_content
                .push(text(display).size(12).color(theme.palette().text).width(Fill));
            if let Some(dex) = helpers::hip3_dex(&sym_key) {
                coin_content = coin_content.push(
                    text(dex.to_string())
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                );
            }
            coin_content = coin_content.push(
                text("Select")
                    .size(10)
                    .color(theme.extended_palette().primary.base.color),
            );

            autocomplete = autocomplete.push(
                button(coin_content)
                    .on_press(Message::PositioningInfoSymbolSelected(id, sym_key))
                    .padding([4, 8])
                    .style(|theme: &Theme, status| {
                        let bg = match status {
                            button::Status::Hovered => {
                                theme.extended_palette().background.strong.color
                            }
                            _ => theme.extended_palette().background.weak.color,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            border: iced::Border {
                                radius: 3.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    })
                    .width(Fill),
            );
        }

        autocomplete
    }

    fn view_positioning_info_controls(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'static, Message> {
        let can_clear = instance.has_active_filters() || instance.error.is_some();
        let side_row =
            PositioningInfoSide::ALL
                .iter()
                .fold(Row::new().spacing(4), |row, &side| {
                    row.push(positioning_control_button(
                        side.label(),
                        instance.side == side,
                        Message::PositioningInfoSideChanged(instance.id, side),
                    ))
                });
        row![
            text("Side")
                .size(10)
                .color(color!(0x888888))
                .width(Length::Fixed(34.0)),
            side_row,
            Space::new().width(Fill),
            positioning_clear_filters_button(instance.id, can_clear),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
    }

    fn view_positioning_info_change_controls(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'static, Message> {
        let timeframe_row = PositioningInfoChangeTimeframe::ALL.iter().fold(
            Row::new().spacing(4),
            |row, &timeframe| {
                row.push(positioning_control_button(
                    timeframe.label(),
                    instance.change_timeframe == timeframe,
                    Message::PositioningInfoChangeTimeframeChanged(instance.id, timeframe),
                ))
            },
        );

        row![
            text("Time")
                .size(10)
                .color(color!(0x888888))
                .width(Length::Fixed(34.0)),
            timeframe_row,
            Space::new().width(Fill),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
    }

    fn view_positioning_info_summary(
        &self,
        data: &TickerPositions,
        instance: &PositioningInfoInstance,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let net_notional = data.total_long_notional - data.total_short_notional;
        let net_color = signed_value_color(net_notional, theme);
        let rows_label = if data.has_more {
            format!("Top {} of {}", POSITIONING_INFO_LIMIT, data.total_count)
        } else {
            data.positions.len().to_string()
        };
        let updated = format_positioning_timestamp(&data.timestamp);
        let last_fetch = instance
            .last_fetch_ms
            .map(|last| {
                format!(
                    "{} ago",
                    helpers::format_relative_time(last, TradingTerminal::now_ms())
                )
            })
            .unwrap_or_else(|| "-".to_string());

        column![
            row![
                helpers::label_value("Long", format_usd_number(data.total_long_notional)),
                helpers::label_value("Short", format_usd_number(data.total_short_notional)),
                helpers::label_value_colored("Net", format_signed_usd(net_notional), net_color),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            row![
                helpers::label_value("Traders", data.total_count.to_string()),
                helpers::label_value("Rows", rows_label),
                helpers::label_value("Updated", updated),
                helpers::label_value("Fetched", last_fetch),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_positioning_info_change_summary(
        &self,
        data: &PerpDeltas,
        instance: &PositioningInfoInstance,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let last_fetch = instance
            .change_last_fetch_ms
            .map(|last| {
                format!(
                    "{} ago",
                    helpers::format_relative_time(last, TradingTerminal::now_ms())
                )
            })
            .unwrap_or_else(|| "-".to_string());
        let shown = data.deltas.len().min(POSITIONING_CHANGE_ROW_LIMIT);
        let rows_label = if shown < data.deltas.len() {
            format!("Showing {shown} of {}", data.deltas.len())
        } else {
            data.deltas.len().to_string()
        };

        row![
            helpers::label_value("Timeframe", instance.change_timeframe.label().to_string()),
            helpers::label_value("Rows", rows_label),
            helpers::label_value("Fetched", last_fetch),
            text(format!("API: {}", data.timeframe))
                .size(10)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_positioning_info_table(
        &self,
        data: &TickerPositions,
        instance: &PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let columns = PositioningInfoColumns::for_width(available_width);
        let live_mark = positioning_live_mark(instance, TradingTerminal::now_ms());
        let mut rows = Column::new()
            .spacing(3)
            .push(positioning_table_header(
                instance.id,
                instance.sort_field,
                instance.sort_direction,
                columns,
                theme,
            ))
            .push(rule::horizontal(1));

        if data.positions.is_empty() {
            rows = rows.push(
                container(
                    text("No positions found")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .padding([8, 0]),
            );
        } else {
            for position in &data.positions {
                rows = rows.push(positioning_position_row(
                    position,
                    self.wallet_display(&position.address),
                    columns,
                    theme,
                    live_mark,
                ));
            }
        }

        scrollable(rows).width(Fill).height(Fill).into()
    }

    fn view_positioning_info_change_table(
        &self,
        data: &PerpDeltas,
        instance: &PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let columns = PositioningChangeColumns::for_width(available_width);
        let live_mark = positioning_live_mark(instance, TradingTerminal::now_ms());
        let sorted = sorted_change_rows(
            &data.deltas,
            instance.change_sort_field,
            instance.change_sort_direction,
            live_mark,
        );
        let mut rows = Column::new()
            .spacing(3)
            .push(positioning_change_table_header(
                instance.id,
                instance.change_sort_field,
                instance.change_sort_direction,
                columns,
                theme,
            ))
            .push(rule::horizontal(1));

        if sorted.is_empty() {
            rows = rows.push(
                container(
                    text("No changes found")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .padding([8, 0]),
            );
        } else {
            for entry in sorted.into_iter().take(POSITIONING_CHANGE_ROW_LIMIT) {
                rows = rows.push(positioning_change_row(
                    entry,
                    self.wallet_display(&entry.address),
                    columns,
                    theme,
                    live_mark,
                ));
            }
        }

        scrollable(rows).width(Fill).height(Fill).into()
    }

    fn positioning_info_symbol_display(&self, symbol: &str) -> String {
        self.exchange_symbols
            .iter()
            .find(|candidate| candidate.key == symbol)
            .map(|candidate| {
                candidate
                    .display_name
                    .as_deref()
                    .unwrap_or(&candidate.ticker)
                    .to_string()
            })
            .unwrap_or_else(|| symbol.to_string())
    }
}

// ---------------------------------------------------------------------------
// Positioning Information Components
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct PositioningInfoColumns {
    trader_width: f32,
    side_width: f32,
    size_width: f32,
    notional_width: f32,
    upnl_width: f32,
    entry_width: f32,
    liq_width: f32,
    funding_width: f32,
    account_width: f32,
    show_entry: bool,
    show_liq: bool,
    show_funding: bool,
    show_account: bool,
}

#[derive(Debug, Clone, Copy)]
struct PositioningChangeColumns {
    trader_width: f32,
    previous_width: f32,
    current_width: f32,
    delta_width: f32,
    current_usd_width: f32,
    delta_usd_width: f32,
}

const POSITIONING_TABLE_CONTENT_PADDING: f32 = 20.0;
const POSITIONING_TABLE_SCROLLBAR_RESERVE: f32 = 14.0;
const POSITIONING_TABLE_CELL_PADDING: f32 = 16.0;
const POSITIONING_TABLE_COLUMN_SPACING: f32 = 6.0;
const POSITIONING_TRADER_MIN_WIDTH: f32 = 112.0;
const POSITIONING_SIDE_WIDTH: f32 = 44.0;
const POSITIONING_SIZE_WIDTH: f32 = 64.0;
const POSITIONING_NOTIONAL_WIDTH: f32 = 76.0;
const POSITIONING_UPNL_WIDTH: f32 = 74.0;
const POSITIONING_ENTRY_WIDTH: f32 = 70.0;
const POSITIONING_LIQ_WIDTH: f32 = 70.0;
const POSITIONING_FUNDING_WIDTH: f32 = 74.0;
const POSITIONING_ACCOUNT_WIDTH: f32 = 76.0;
const POSITIONING_TRADER_WEIGHT: f32 = 2.4;
const POSITIONING_SIDE_WEIGHT: f32 = 0.7;
const POSITIONING_SIZE_WEIGHT: f32 = 1.0;
const POSITIONING_NOTIONAL_WEIGHT: f32 = 1.15;
const POSITIONING_UPNL_WEIGHT: f32 = 1.15;
const POSITIONING_ENTRY_WEIGHT: f32 = 1.0;
const POSITIONING_LIQ_WEIGHT: f32 = 1.0;
const POSITIONING_FUNDING_WEIGHT: f32 = 1.1;
const POSITIONING_ACCOUNT_WEIGHT: f32 = 1.15;
const POSITIONING_TRADER_COMPACT_ACTIONS_MIN_WIDTH: f32 = 168.0;
const POSITIONING_TRADER_FULL_ACTIONS_MIN_WIDTH: f32 = 240.0;
const POSITIONING_TRADER_COMPACT_ACTIONS_WIDTH: f32 = 42.0;
const POSITIONING_TRADER_FULL_ACTIONS_WIDTH: f32 = 106.0;
const POSITIONING_CHANGE_TRADER_MIN_WIDTH: f32 = 132.0;
const POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH: f32 =
    POSITIONING_CHANGE_TRADER_MIN_WIDTH;
const POSITIONING_CHANGE_PREVIOUS_WIDTH: f32 = 76.0;
const POSITIONING_CHANGE_CURRENT_WIDTH: f32 = 76.0;
const POSITIONING_CHANGE_DELTA_WIDTH: f32 = 76.0;
const POSITIONING_CHANGE_CURRENT_USD_WIDTH: f32 = 84.0;
const POSITIONING_CHANGE_DELTA_USD_WIDTH: f32 = 84.0;
const POSITIONING_CHANGE_TRADER_WEIGHT: f32 = 2.6;
const POSITIONING_CHANGE_NUMERIC_WEIGHT: f32 = 1.0;

impl PositioningInfoColumns {
    fn for_width(width: f32) -> Self {
        let content_width = Self::available_content_width(width);
        let base_fixed_width = POSITIONING_SIDE_WIDTH
            + POSITIONING_SIZE_WIDTH
            + POSITIONING_NOTIONAL_WIDTH
            + POSITIONING_UPNL_WIDTH;
        let base_width_without_trader = POSITIONING_TABLE_CELL_PADDING
            + base_fixed_width
            + POSITIONING_TABLE_COLUMN_SPACING * 4.0;
        let available_for_trader = (content_width - base_width_without_trader).max(0.0);
        let trader_width = if available_for_trader < POSITIONING_TRADER_MIN_WIDTH {
            available_for_trader
        } else {
            POSITIONING_TRADER_MIN_WIDTH
        };
        let mut used_width = base_width_without_trader + trader_width;
        let mut include_column = |column_width: f32| {
            let next_width = used_width + POSITIONING_TABLE_COLUMN_SPACING + column_width;
            if next_width <= content_width {
                used_width = next_width;
                true
            } else {
                false
            }
        };

        let show_entry = include_column(POSITIONING_ENTRY_WIDTH);
        let show_liq = include_column(POSITIONING_LIQ_WIDTH);
        let show_funding = include_column(POSITIONING_FUNDING_WIDTH);
        let show_account = include_column(POSITIONING_ACCOUNT_WIDTH);

        let mut columns = Self {
            trader_width,
            side_width: POSITIONING_SIDE_WIDTH,
            size_width: POSITIONING_SIZE_WIDTH,
            notional_width: POSITIONING_NOTIONAL_WIDTH,
            upnl_width: POSITIONING_UPNL_WIDTH,
            entry_width: POSITIONING_ENTRY_WIDTH,
            liq_width: POSITIONING_LIQ_WIDTH,
            funding_width: POSITIONING_FUNDING_WIDTH,
            account_width: POSITIONING_ACCOUNT_WIDTH,
            show_entry,
            show_liq,
            show_funding,
            show_account,
        };
        columns.distribute_extra_width((content_width - columns.total_width()).max(0.0));
        columns
    }

    fn available_content_width(width: f32) -> f32 {
        if width.is_finite() {
            (width - POSITIONING_TABLE_CONTENT_PADDING - POSITIONING_TABLE_SCROLLBAR_RESERVE)
                .max(0.0)
        } else {
            0.0
        }
    }

    fn visible_column_count(self) -> usize {
        5 + usize::from(self.show_entry)
            + usize::from(self.show_liq)
            + usize::from(self.show_funding)
            + usize::from(self.show_account)
    }

    fn total_width(self) -> f32 {
        let mut optional_width = 0.0;
        if self.show_entry {
            optional_width += self.entry_width;
        }
        if self.show_liq {
            optional_width += self.liq_width;
        }
        if self.show_funding {
            optional_width += self.funding_width;
        }
        if self.show_account {
            optional_width += self.account_width;
        }
        let gap_count = self.visible_column_count().saturating_sub(1) as f32;
        POSITIONING_TABLE_CELL_PADDING
            + self.trader_width
            + self.side_width
            + self.size_width
            + self.notional_width
            + self.upnl_width
            + optional_width
            + POSITIONING_TABLE_COLUMN_SPACING * gap_count
    }

    fn distribute_extra_width(&mut self, extra: f32) {
        if extra <= 0.0 {
            return;
        }

        let total_weight = POSITIONING_TRADER_WEIGHT
            + POSITIONING_SIDE_WEIGHT
            + POSITIONING_SIZE_WEIGHT
            + POSITIONING_NOTIONAL_WEIGHT
            + POSITIONING_UPNL_WEIGHT
            + if self.show_entry {
                POSITIONING_ENTRY_WEIGHT
            } else {
                0.0
            }
            + if self.show_liq {
                POSITIONING_LIQ_WEIGHT
            } else {
                0.0
            }
            + if self.show_funding {
                POSITIONING_FUNDING_WEIGHT
            } else {
                0.0
            }
            + if self.show_account {
                POSITIONING_ACCOUNT_WEIGHT
            } else {
                0.0
            };

        self.trader_width += extra * POSITIONING_TRADER_WEIGHT / total_weight;
        self.side_width += extra * POSITIONING_SIDE_WEIGHT / total_weight;
        self.size_width += extra * POSITIONING_SIZE_WEIGHT / total_weight;
        self.notional_width += extra * POSITIONING_NOTIONAL_WEIGHT / total_weight;
        self.upnl_width += extra * POSITIONING_UPNL_WEIGHT / total_weight;
        if self.show_entry {
            self.entry_width += extra * POSITIONING_ENTRY_WEIGHT / total_weight;
        }
        if self.show_liq {
            self.liq_width += extra * POSITIONING_LIQ_WEIGHT / total_weight;
        }
        if self.show_funding {
            self.funding_width += extra * POSITIONING_FUNDING_WEIGHT / total_weight;
        }
        if self.show_account {
            self.account_width += extra * POSITIONING_ACCOUNT_WEIGHT / total_weight;
        }
    }
}

impl PositioningChangeColumns {
    fn for_width(width: f32) -> Self {
        let content_width = PositioningInfoColumns::available_content_width(width);
        let fixed_width = POSITIONING_CHANGE_PREVIOUS_WIDTH
            + POSITIONING_CHANGE_CURRENT_WIDTH
            + POSITIONING_CHANGE_DELTA_WIDTH
            + POSITIONING_CHANGE_CURRENT_USD_WIDTH
            + POSITIONING_CHANGE_DELTA_USD_WIDTH;
        let base_width_without_trader = POSITIONING_TABLE_CELL_PADDING
            + fixed_width
            + POSITIONING_TABLE_COLUMN_SPACING * 5.0;
        let available_for_trader = (content_width - base_width_without_trader).max(0.0);
        let trader_width = if available_for_trader < POSITIONING_CHANGE_TRADER_MIN_WIDTH {
            available_for_trader
        } else {
            POSITIONING_CHANGE_TRADER_MIN_WIDTH
        };

        let mut columns = Self {
            trader_width,
            previous_width: POSITIONING_CHANGE_PREVIOUS_WIDTH,
            current_width: POSITIONING_CHANGE_CURRENT_WIDTH,
            delta_width: POSITIONING_CHANGE_DELTA_WIDTH,
            current_usd_width: POSITIONING_CHANGE_CURRENT_USD_WIDTH,
            delta_usd_width: POSITIONING_CHANGE_DELTA_USD_WIDTH,
        };
        columns.distribute_extra_width((content_width - columns.total_width()).max(0.0));
        columns
    }

    fn total_width(self) -> f32 {
        POSITIONING_TABLE_CELL_PADDING
            + self.trader_width
            + self.previous_width
            + self.current_width
            + self.delta_width
            + self.current_usd_width
            + self.delta_usd_width
            + POSITIONING_TABLE_COLUMN_SPACING * 5.0
    }

    fn distribute_extra_width(&mut self, extra: f32) {
        if extra <= 0.0 {
            return;
        }

        let total_weight =
            POSITIONING_CHANGE_TRADER_WEIGHT + POSITIONING_CHANGE_NUMERIC_WEIGHT * 5.0;
        self.trader_width += extra * POSITIONING_CHANGE_TRADER_WEIGHT / total_weight;
        let numeric_extra = extra * POSITIONING_CHANGE_NUMERIC_WEIGHT / total_weight;
        self.previous_width += numeric_extra;
        self.current_width += numeric_extra;
        self.delta_width += numeric_extra;
        self.current_usd_width += numeric_extra;
        self.delta_usd_width += numeric_extra;
    }
}

fn positioning_table_header(
    id: PositioningInfoId,
    sort_field: PositioningInfoSortField,
    sort_direction: config::SortDirection,
    columns: PositioningInfoColumns,
    theme: &Theme,
) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let mut header = Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([0, 8])
        .push(header_cell_aligned(
            "Trader",
            Length::Fixed(columns.trader_width),
            muted,
            Horizontal::Left,
        ))
        .push(header_cell_aligned(
            "Side",
            Length::Fixed(columns.side_width),
            muted,
            Horizontal::Left,
        ))
        .push(sort_header_cell(
            "Size",
            PositioningInfoSortField::Size,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.size_width),
            muted,
        ))
        .push(sort_header_cell(
            "Notional",
            PositioningInfoSortField::NotionalSize,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.notional_width),
            muted,
        ))
        .push(sort_header_cell(
            "uPnL",
            PositioningInfoSortField::UnrealizedPnl,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.upnl_width),
            muted,
        ));

    if columns.show_entry {
        header = header.push(header_cell("Entry", Length::Fixed(columns.entry_width), muted));
    }
    if columns.show_liq {
        header = header.push(header_cell("Liq", Length::Fixed(columns.liq_width), muted));
    }
    if columns.show_funding {
        header = header.push(header_cell(
            "Funding",
            Length::Fixed(columns.funding_width),
            muted,
        ));
    }
    if columns.show_account {
        header = header.push(sort_header_cell(
            "Account",
            PositioningInfoSortField::AccountValue,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.account_width),
            muted,
        ));
    }

    header.into()
}

fn positioning_position_row(
    position: &TickerPositionEntry,
    wallet_display: WalletDisplay,
    columns: PositioningInfoColumns,
    theme: &Theme,
    live_mark: Option<f64>,
) -> Element<'static, Message> {
    let side = position_side_label(position.size);
    let side_color = position_side_color(position.size, theme);
    let notional =
        positioning_live_notional(position, live_mark).unwrap_or(position.notional_size);
    let unrealized_pnl =
        positioning_live_unrealized_pnl(position, live_mark).unwrap_or(position.unrealized_pnl);
    let pnl_color = signed_value_color(unrealized_pnl, theme);
    let funding_color = signed_value_color(position.funding_pnl, theme);

    let mut row = Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([4, 8])
        .align_y(Alignment::Center)
        .push(positioning_trader_cell(
            &position.address,
            wallet_display,
            columns.trader_width,
            POSITIONING_TRADER_COMPACT_ACTIONS_MIN_WIDTH,
            theme,
        ))
        .push(value_cell(
            side,
            Length::Fixed(columns.side_width),
            side_color,
            false,
        ))
        .push(value_cell(
            helpers::format_size(position.size.abs()),
            Length::Fixed(columns.size_width),
            theme.palette().text,
            true,
        ))
        .push(value_cell(
            format_usd_number(notional.abs()),
            Length::Fixed(columns.notional_width),
            theme.palette().text,
            true,
        ))
        .push(value_cell(
            format_signed_usd(unrealized_pnl),
            Length::Fixed(columns.upnl_width),
            pnl_color,
            true,
        ));

    if columns.show_entry {
        row = row.push(value_cell(
            format_price_number(position.entry_price),
            Length::Fixed(columns.entry_width),
            theme.palette().text,
            true,
        ));
    }
    if columns.show_liq {
        row = row.push(value_cell(
            position
                .liquidation_price
                .map(format_price_number)
                .unwrap_or_else(|| "-".to_string()),
            Length::Fixed(columns.liq_width),
            theme.palette().text,
            true,
        ));
    }
    if columns.show_funding {
        row = row.push(value_cell(
            format_signed_usd(position.funding_pnl),
            Length::Fixed(columns.funding_width),
            funding_color,
            true,
        ));
    }
    if columns.show_account {
        row = row.push(value_cell(
            format_usd_number(position.account_value),
            Length::Fixed(columns.account_width),
            theme.palette().text,
            true,
        ));
    }

    container(row)
        .width(Fill)
        .style(move |_theme: &Theme| {
            use iced::gradient;
            let mut base_color = side_color;
            base_color.a = 0.15;
            iced::widget::container::Style {
                background: Some(
                    gradient::Linear::new(iced::Degrees(90.0))
                        .add_stop(0.0, base_color)
                        .add_stop(0.20, iced::Color::TRANSPARENT)
                        .add_stop(1.0, iced::Color::TRANSPARENT)
                        .into(),
                ),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn positioning_change_table_header(
    id: PositioningInfoId,
    sort_field: PositioningInfoChangeSortField,
    sort_direction: config::SortDirection,
    columns: PositioningChangeColumns,
    theme: &Theme,
) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([0, 8])
        .push(change_sort_header_cell(
            "Trader",
            PositioningInfoChangeSortField::Trader,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.trader_width),
            muted,
            Horizontal::Left,
        ))
        .push(change_sort_header_cell(
            "Previous",
            PositioningInfoChangeSortField::Previous,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.previous_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            "Current",
            PositioningInfoChangeSortField::Current,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.current_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            "\u{0394} Change",
            PositioningInfoChangeSortField::Change,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.delta_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            "Current $",
            PositioningInfoChangeSortField::CurrentUsd,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.current_usd_width),
            muted,
            Horizontal::Right,
        ))
        .push(change_sort_header_cell(
            "Change $",
            PositioningInfoChangeSortField::ChangeUsd,
            id,
            sort_field,
            sort_direction,
            Length::Fixed(columns.delta_usd_width),
            muted,
            Horizontal::Right,
        ))
        .into()
}

fn positioning_change_row(
    entry: &PerpDeltaEntry,
    wallet_display: WalletDisplay,
    columns: PositioningChangeColumns,
    theme: &Theme,
    live_mark: Option<f64>,
) -> Element<'static, Message> {
    let previous = positioning_previous_change_size(entry);
    let previous_color = previous
        .map(|value| signed_value_color(value, theme))
        .unwrap_or_else(|| theme.extended_palette().background.weak.text);
    let current_color = signed_value_color(entry.current, theme);
    let delta_color = signed_value_color(entry.delta, theme);
    let current_usd = positioning_live_change_usd(entry.current, live_mark)
        .map(format_signed_usd)
        .unwrap_or_else(|| "-".to_string());
    let delta_usd = positioning_live_change_usd(entry.delta, live_mark)
        .map(format_signed_usd)
        .unwrap_or_else(|| "-".to_string());

    let row = Row::new()
        .spacing(POSITIONING_TABLE_COLUMN_SPACING)
        .padding([4, 8])
        .align_y(Alignment::Center)
        .push(positioning_trader_cell(
            &entry.address,
            wallet_display,
            columns.trader_width,
            POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH,
            theme,
        ))
        .push(value_cell(
            previous
                .map(|value| format_signed_size(value, false))
                .unwrap_or_else(|| "-".to_string()),
            Length::Fixed(columns.previous_width),
            previous_color,
            true,
        ))
        .push(value_cell(
            format_signed_size(entry.current, false),
            Length::Fixed(columns.current_width),
            current_color,
            true,
        ))
        .push(value_cell(
            format_signed_size(entry.delta, true),
            Length::Fixed(columns.delta_width),
            delta_color,
            true,
        ))
        .push(value_cell(
            current_usd,
            Length::Fixed(columns.current_usd_width),
            current_color,
            true,
        ))
        .push(value_cell(
            delta_usd,
            Length::Fixed(columns.delta_usd_width),
            delta_color,
            true,
        ));

    container(row)
        .width(Fill)
        .style(move |_theme: &Theme| {
            use iced::gradient;
            let mut base_color = delta_color;
            base_color.a = 0.12;
            iced::widget::container::Style {
                background: Some(
                    gradient::Linear::new(iced::Degrees(90.0))
                        .add_stop(0.0, base_color)
                        .add_stop(0.20, iced::Color::TRANSPARENT)
                        .add_stop(1.0, iced::Color::TRANSPARENT)
                        .into(),
                ),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn positioning_trader_cell(
    address: &str,
    wallet_display: WalletDisplay,
    width: f32,
    compact_actions_min_width: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    let identity_label = position_identity(wallet_display);
    let address = address.to_string();
    let (show_actions, show_full_actions) =
        positioning_trader_action_visibility(width, compact_actions_min_width);
    let action_width = if show_actions {
        if show_full_actions {
            POSITIONING_TRADER_FULL_ACTIONS_WIDTH
        } else {
            POSITIONING_TRADER_COMPACT_ACTIONS_WIDTH
        }
    } else {
        0.0
    };
    let identity_width = (width - action_width).max(0.0);
    let label_limit = trader_text_limit(identity_width, 34);

    let identity_content = text(truncate_ascii(&identity_label, label_limit))
        .size(11)
        .color(theme.palette().text)
        .width(Fill);

    let identity_button = button(identity_content)
        .on_press(Message::CopyToClipboard(address.clone()))
        .padding(0)
        .style(|theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => Some(
                    Color {
                        a: 0.18,
                        ..theme.extended_palette().background.weak.color
                    }
                    .into(),
                ),
                _ => None,
            };
            button::Style {
                background,
                ..Default::default()
            }
        })
        .width(Fill);
    let identity: Element<'static, Message> = tooltip(
        identity_button,
        text(format!("Copy {address}"))
            .size(10)
            .font(Font::MONOSPACE),
        tooltip::Position::Top,
    )
    .into();

    let mut content = row![identity]
        .spacing(3)
        .align_y(Alignment::Center)
        .width(Fill);
    if show_actions {
        content = content
            .push(positioning_trader_action_button(
                if show_full_actions {
                    "Details"
                } else {
                    "\u{2197}"
                },
                "Open detachable wallet details",
                Message::OpenWalletDetailsWindow(address.clone()),
                show_full_actions,
            ))
            .push(positioning_trader_action_button(
                if show_full_actions { "Ghost" } else { "G" },
                "Open in ghost mode",
                Message::GhostWallet(address),
                show_full_actions,
            ));
    }

    container(content)
        .width(Length::Fixed(width))
        .padding([1, 0])
        .into()
}

fn positioning_trader_action_visibility(
    width: f32,
    compact_actions_min_width: f32,
) -> (bool, bool) {
    (
        width >= compact_actions_min_width,
        width >= POSITIONING_TRADER_FULL_ACTIONS_MIN_WIDTH,
    )
}

fn header_cell(label: &'static str, width: Length, color: Color) -> Element<'static, Message> {
    header_cell_aligned(label, width, color, Horizontal::Right)
}

fn header_cell_aligned(
    label: &'static str,
    width: Length,
    color: Color,
    alignment: Horizontal,
) -> Element<'static, Message> {
    text(label)
        .size(10)
        .color(color)
        .width(width)
        .align_x(alignment)
        .into()
}

fn sort_header_cell(
    label: &'static str,
    field: PositioningInfoSortField,
    id: PositioningInfoId,
    sort_field: PositioningInfoSortField,
    sort_direction: config::SortDirection,
    width: Length,
    color: Color,
) -> Element<'static, Message> {
    let is_active = sort_field == field;
    let mut content = Row::new().spacing(2).align_y(Alignment::Center).push(
        text(label)
            .size(10)
            .color(color)
            .width(Fill)
            .align_x(Horizontal::Right),
    );
    if is_active {
        let icon = if sort_direction == config::SortDirection::Ascending {
            "\u{2191}"
        } else {
            "\u{2193}"
        };
        content = content.push(text(icon).size(10).color(color));
    }

    button(content)
        .on_press(Message::PositioningInfoSortChanged(id, field))
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .padding(0)
        .width(width)
        .into()
}

#[allow(clippy::too_many_arguments)]
fn change_sort_header_cell(
    label: &'static str,
    field: PositioningInfoChangeSortField,
    id: PositioningInfoId,
    sort_field: PositioningInfoChangeSortField,
    sort_direction: config::SortDirection,
    width: Length,
    color: Color,
    alignment: Horizontal,
) -> Element<'static, Message> {
    let is_active = sort_field == field;
    let mut content = Row::new().spacing(2).align_y(Alignment::Center).push(
        text(label)
            .size(10)
            .color(color)
            .width(Fill)
            .align_x(alignment),
    );
    if is_active {
        let icon = if sort_direction == config::SortDirection::Ascending {
            "\u{2191}"
        } else {
            "\u{2193}"
        };
        content = content.push(text(icon).size(10).color(color));
    }

    button(content)
        .on_press(Message::PositioningInfoChangeSortChanged(id, field))
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .padding(0)
        .width(width)
        .into()
}

fn value_cell(
    value: impl ToString,
    width: Length,
    color: Color,
    align_right: bool,
) -> Element<'static, Message> {
    let cell = text(value.to_string())
        .size(11)
        .font(Font::MONOSPACE)
        .color(color)
        .width(width);
    if align_right {
        cell.align_x(Horizontal::Right).into()
    } else {
        cell.into()
    }
}

fn positioning_trader_action_button(
    label: &'static str,
    tooltip_label: &'static str,
    msg: Message,
    full: bool,
) -> Element<'static, Message> {
    let button_width = if full { 50.0 } else { 18.0 };
    tooltip(
        button(text(label).size(10).font(Font::MONOSPACE).center())
            .on_press(msg)
            .padding([0, 4])
            .width(Length::Fixed(button_width))
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().primary,
                    border: iced::Border {
                        radius: 3.0.into(),
                        width: 1.0,
                        color: Color {
                            a: 0.45,
                            ..theme.palette().primary
                        },
                    },
                    ..Default::default()
                }
            }),
        text(tooltip_label).size(10),
        tooltip::Position::Top,
    )
    .into()
}

fn positioning_control_button(
    label: &'static str,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label).size(10).center())
        .on_press(msg)
        .padding([2, 7])
        .style(move |theme: &Theme, status| {
            let bg = if active {
                theme.extended_palette().background.strong.color
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if active {
                    theme.palette().text
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        Color {
                            a: 0.4,
                            ..theme.palette().primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}

fn positioning_clear_filters_button(
    id: PositioningInfoId,
    active: bool,
) -> Element<'static, Message> {
    let mut clear_button = button(text("Clear filters").size(10).center())
        .padding([2, 7])
        .style(move |theme: &Theme, status| {
            let text_color = if active {
                theme.extended_palette().primary.base.color
            } else {
                theme.extended_palette().background.weak.text
            };
            let bg = match status {
                button::Status::Hovered if active => {
                    theme.extended_palette().background.weak.color
                }
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Color {
                    a: if active { 1.0 } else { 0.55 },
                    ..text_color
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    width: 1.0,
                    color: Color {
                        a: if active { 0.35 } else { 0.18 },
                        ..text_color
                    },
                },
                ..Default::default()
            }
        });
    if active {
        clear_button = clear_button.on_press(Message::ClearPositioningInfoFilters(id));
    }
    clear_button.into()
}

fn positioning_navigation_button(
    id: PositioningInfoId,
    page: PositioningInfoPage,
    active: bool,
) -> Element<'static, Message> {
    button(text(page.label()).size(11).center())
        .on_press(Message::PositioningInfoPageChanged(id, page))
        .padding([3, 9])
        .style(move |theme: &Theme, status| {
            let bg = if active {
                theme.extended_palette().background.strong.color
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        Color {
                            a: 0.35,
                            ..theme.palette().primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}

// ---------------------------------------------------------------------------
// Positioning Information Formatting
// ---------------------------------------------------------------------------

fn positioning_symbol_matches(symbol: &ExchangeSymbol, query: &str) -> bool {
    symbol.key.to_lowercase().contains(query)
        || symbol.ticker.to_lowercase().contains(query)
        || symbol
            .display_name
            .as_deref()
            .is_some_and(|display| display.to_lowercase().contains(query))
        || symbol
            .keywords
            .iter()
            .any(|keyword| keyword.to_lowercase().contains(query))
}

fn position_identity(wallet_display: WalletDisplay) -> String {
    wallet_display.primary
}

fn trader_text_limit(width: f32, max_chars: usize) -> usize {
    let estimated_chars = ((width.max(0.0) - 8.0).max(0.0) / 6.4).floor() as usize;
    estimated_chars.clamp(8, max_chars)
}

fn truncate_ascii(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    truncated.push_str("...");
    truncated
}

const POSITIONING_LIVE_MARK_MAX_AGE_MS: u64 = 15_000;

fn positioning_live_mark(instance: &PositioningInfoInstance, now_ms: u64) -> Option<f64> {
    let updated_at = instance.asset_ctx_updated_at_ms?;
    if now_ms.checked_sub(updated_at)? > POSITIONING_LIVE_MARK_MAX_AGE_MS {
        return None;
    }
    let ctx = instance.asset_ctx.as_ref()?;
    parse_live_ctx_price(ctx.mark_px.as_deref())
        .or_else(|| parse_live_ctx_price(ctx.mid_px.as_deref()))
}

fn parse_live_ctx_price(value: Option<&str>) -> Option<f64> {
    value?
        .parse::<f64>()
        .ok()
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn positioning_live_notional(
    position: &TickerPositionEntry,
    live_mark: Option<f64>,
) -> Option<f64> {
    let mark = live_mark?;
    if !mark.is_finite() || mark <= 0.0 {
        return None;
    }
    position
        .size
        .is_finite()
        .then_some(position.size.abs() * mark)
}

fn positioning_live_unrealized_pnl(
    position: &TickerPositionEntry,
    live_mark: Option<f64>,
) -> Option<f64> {
    let mark = live_mark?;
    if mark.is_finite()
        && mark > 0.0
        && position.size.is_finite()
        && position.entry_price.is_finite()
        && position.entry_price > 0.0
    {
        Some(position.size * (mark - position.entry_price))
    } else {
        None
    }
}

fn positioning_live_change_usd(value: f64, live_mark: Option<f64>) -> Option<f64> {
    let mark = live_mark?;
    if mark.is_finite() && mark > 0.0 && value.is_finite() {
        Some(value * mark)
    } else {
        None
    }
}

fn positioning_previous_change_size(entry: &PerpDeltaEntry) -> Option<f64> {
    let previous = entry.current - entry.delta;
    previous.is_finite().then_some(previous)
}

fn sorted_change_rows(
    deltas: &[PerpDeltaEntry],
    sort_field: PositioningInfoChangeSortField,
    sort_direction: config::SortDirection,
    live_mark: Option<f64>,
) -> Vec<&PerpDeltaEntry> {
    let mut rows: Vec<&PerpDeltaEntry> = deltas.iter().collect();
    rows.sort_by(|a, b| {
        let ordering = match sort_field {
            PositioningInfoChangeSortField::Trader => a.address.cmp(&b.address),
            PositioningInfoChangeSortField::Previous => optional_number_cmp_directional(
                positioning_previous_change_size(a),
                positioning_previous_change_size(b),
                sort_direction,
            ),
            PositioningInfoChangeSortField::Current => optional_number_cmp_directional(
                finite_number(a.current),
                finite_number(b.current),
                sort_direction,
            ),
            PositioningInfoChangeSortField::Change => optional_number_cmp_directional(
                finite_number(a.delta.abs()),
                finite_number(b.delta.abs()),
                sort_direction,
            ),
            PositioningInfoChangeSortField::CurrentUsd => optional_number_cmp_directional(
                positioning_live_change_usd(a.current, live_mark),
                positioning_live_change_usd(b.current, live_mark),
                sort_direction,
            ),
            PositioningInfoChangeSortField::ChangeUsd => optional_number_cmp_directional(
                positioning_live_change_usd(a.delta, live_mark).map(f64::abs),
                positioning_live_change_usd(b.delta, live_mark).map(f64::abs),
                sort_direction,
            ),
        };
        let ordering = if sort_field == PositioningInfoChangeSortField::Trader
            && sort_direction == config::SortDirection::Descending
        {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| a.address.cmp(&b.address))
    });
    rows
}

fn optional_number_cmp_directional(
    a: Option<f64>,
    b: Option<f64>,
    direction: config::SortDirection,
) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            let ordering = a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
            if direction == config::SortDirection::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn finite_number(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn position_side_label(size: f64) -> &'static str {
    if size > 0.0 {
        "\u{2191} Long"
    } else if size < 0.0 {
        "\u{2193} Short"
    } else {
        "Flat"
    }
}

fn position_side_color(size: f64, theme: &Theme) -> Color {
    if size > 0.0 {
        theme.palette().success
    } else if size < 0.0 {
        theme.palette().danger
    } else {
        theme.extended_palette().background.weak.text
    }
}

fn signed_value_color(value: f64, theme: &Theme) -> Color {
    if value > 0.0 {
        theme.palette().success
    } else if value < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}

fn format_usd_number(value: f64) -> String {
    if value.is_finite() {
        helpers::format_usd(&format!("{value:.2}"))
    } else {
        "-".to_string()
    }
}

fn format_signed_usd(value: f64) -> String {
    if value.is_finite() {
        account_metrics::format_signed_usd_value(value)
    } else {
        "-".to_string()
    }
}

fn format_price_number(value: f64) -> String {
    if value.is_finite() && value > 0.0 {
        helpers::format_price(value)
    } else {
        "-".to_string()
    }
}

fn format_signed_size(value: f64, plus_for_positive: bool) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let size = helpers::format_size(value.abs());
    if value > 0.0 && plus_for_positive {
        format!("+{size}")
    } else if value < 0.0 {
        format!("-{size}")
    } else {
        size
    }
}

fn format_positioning_timestamp(timestamp: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%b %d, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| timestamp.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AssetContext;

    fn sample_position() -> TickerPositionEntry {
        TickerPositionEntry {
            address: "0xabc0000000000000000000000000000000001234".to_string(),
            display_name: None,
            label: Some("Desk A".to_string()),
            tag: Some("macro".to_string()),
            verified: Some(true),
            copy_score: Some(42.0),
            size: 10.0,
            notional_size: 1000.0,
            entry_price: 25.0,
            liquidation_price: None,
            unrealized_pnl: 15.0,
            funding_pnl: -1.0,
            account_value: 5000.0,
        }
    }

    fn asset_ctx(mark_px: Option<&str>, mid_px: Option<&str>) -> AssetContext {
        AssetContext {
            funding: None,
            open_interest: None,
            oracle_px: None,
            mark_px: mark_px.map(str::to_string),
            mid_px: mid_px.map(str::to_string),
            prev_day_px: None,
            day_ntl_vlm: None,
            impact_pxs: None,
        }
    }

    fn delta(address: &str, current: f64, change: f64) -> PerpDeltaEntry {
        PerpDeltaEntry {
            address: address.to_string(),
            current,
            delta: change,
        }
    }

    #[test]
    fn identity_uses_local_wallet_label_when_available() {
        let wallet_display = WalletDisplay {
            primary: "Local Desk".to_string(),
            secondary: "0xabc0...1234".to_string(),
            has_label: true,
        };

        let name = position_identity(wallet_display);

        assert_eq!(name, "Local Desk");
    }

    #[test]
    fn identity_ignores_api_wallet_labels_without_local_label() {
        let position = sample_position();
        let wallet_display = WalletDisplay {
            primary: "0xabc0...1234".to_string(),
            secondary: position.address.clone(),
            has_label: false,
        };

        let name = position_identity(wallet_display);

        assert_eq!(name, "0xabc0...1234");
    }

    #[test]
    fn numeric_formatters_reject_nonfinite_values() {
        assert_eq!(format_usd_number(f64::NAN), "-");
        assert_eq!(format_signed_usd(f64::INFINITY), "-");
        assert_eq!(format_price_number(0.0), "-");
    }

    #[test]
    fn positioning_live_mark_prefers_fresh_mark_context() {
        let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
        instance.asset_ctx = Some(asset_ctx(Some("31"), Some("30.5")));
        instance.asset_ctx_updated_at_ms = Some(1_000);

        assert_eq!(positioning_live_mark(&instance, 2_000), Some(31.0));
    }

    #[test]
    fn positioning_live_mark_rejects_stale_or_invalid_context() {
        let mut instance = PositioningInfoInstance::new(7, "HYPE".to_string());
        instance.asset_ctx = Some(asset_ctx(Some("bad"), Some("30.5")));
        instance.asset_ctx_updated_at_ms = Some(1_000);

        assert_eq!(positioning_live_mark(&instance, 2_000), Some(30.5));
        assert_eq!(
            positioning_live_mark(&instance, 1_000 + POSITIONING_LIVE_MARK_MAX_AGE_MS + 1),
            None
        );
    }

    #[test]
    fn positioning_live_row_values_use_mark_without_mutating_size() {
        let position = sample_position();

        assert_eq!(
            positioning_live_notional(&position, Some(31.0)),
            Some(310.0)
        );
        assert_eq!(
            positioning_live_unrealized_pnl(&position, Some(31.0)),
            Some(60.0)
        );
    }

    #[test]
    fn positioning_change_usd_uses_live_mark() {
        assert_eq!(positioning_live_change_usd(-2.5, Some(20.0)), Some(-50.0));
        assert_eq!(positioning_live_change_usd(2.5, None), None);
        assert_eq!(positioning_live_change_usd(f64::NAN, Some(20.0)), None);
    }

    #[test]
    fn positioning_change_previous_size_is_derived_from_current_and_delta() {
        assert_eq!(
            positioning_previous_change_size(&delta("0xaaa", 0.0, -50.0)),
            Some(50.0)
        );
        assert_eq!(
            positioning_previous_change_size(&delta("0xbbb", 65.5, 65.5)),
            Some(0.0)
        );
        assert_eq!(
            positioning_previous_change_size(&delta("0xccc", -100.0, 30.0)),
            Some(-130.0)
        );
    }

    #[test]
    fn positioning_change_sort_defaults_to_largest_absolute_change() {
        let rows = vec![
            delta("0xaaa", 100.0, -5.0),
            delta("0xbbb", 10.0, 50.0),
            delta("0xccc", -10.0, -75.0),
        ];

        let sorted = sorted_change_rows(
            &rows,
            PositioningInfoChangeSortField::Change,
            config::SortDirection::Descending,
            Some(10.0),
        );

        assert_eq!(sorted[0].address, "0xccc");
        assert_eq!(sorted[1].address, "0xbbb");
        assert_eq!(sorted[2].address, "0xaaa");
    }

    #[test]
    fn positioning_change_sort_can_use_derived_previous_size() {
        let rows = vec![
            delta("0xaaa", 0.0, -10.0),
            delta("0xbbb", 30.0, 5.0),
            delta("0xccc", -20.0, 5.0),
        ];

        let sorted = sorted_change_rows(
            &rows,
            PositioningInfoChangeSortField::Previous,
            config::SortDirection::Descending,
            Some(10.0),
        );

        assert_eq!(sorted[0].address, "0xbbb");
        assert_eq!(sorted[1].address, "0xaaa");
        assert_eq!(sorted[2].address, "0xccc");
    }

    #[test]
    fn positioning_change_sort_keeps_invalid_values_last() {
        let rows = vec![
            delta("0xaaa", f64::NAN, 1.0),
            delta("0xbbb", 5.0, 1.0),
            delta("0xccc", 10.0, 1.0),
        ];

        let descending = sorted_change_rows(
            &rows,
            PositioningInfoChangeSortField::Current,
            config::SortDirection::Descending,
            Some(10.0),
        );
        let ascending = sorted_change_rows(
            &rows,
            PositioningInfoChangeSortField::Current,
            config::SortDirection::Ascending,
            Some(10.0),
        );

        assert_eq!(descending[0].address, "0xccc");
        assert_eq!(descending[2].address, "0xaaa");
        assert_eq!(ascending[0].address, "0xbbb");
        assert_eq!(ascending[2].address, "0xaaa");
    }

    #[test]
    fn positioning_columns_expand_to_span_wide_panes() {
        let width = 1_200.0;
        let columns = PositioningInfoColumns::for_width(width);
        let content_width = PositioningInfoColumns::available_content_width(width);

        assert!((columns.total_width() - content_width).abs() < 0.01);
        assert!(columns.trader_width > POSITIONING_TRADER_MIN_WIDTH);
        assert!(columns.size_width > POSITIONING_SIZE_WIDTH);
        assert!(columns.show_entry);
        assert!(columns.show_liq);
        assert!(columns.show_funding);
        assert!(columns.show_account);
    }

    #[test]
    fn positioning_columns_shrink_trader_width_on_narrow_panes() {
        let width = 380.0;
        let columns = PositioningInfoColumns::for_width(width);
        let content_width = PositioningInfoColumns::available_content_width(width);

        assert!((columns.total_width() - content_width).abs() < 0.01);
        assert!(columns.trader_width < POSITIONING_TRADER_MIN_WIDTH);
        assert!(!columns.show_entry);
        assert!(!columns.show_liq);
        assert!(!columns.show_funding);
        assert!(!columns.show_account);
    }

    #[test]
    fn positioning_change_columns_reserve_scrollbar_width() {
        let width = 900.0;
        let columns = PositioningChangeColumns::for_width(width);
        let content_width = PositioningInfoColumns::available_content_width(width);

        assert!((columns.total_width() - content_width).abs() < 0.01);
        assert!(columns.trader_width > POSITIONING_CHANGE_TRADER_MIN_WIDTH);
    }

    #[test]
    fn positioning_change_trader_column_shows_compact_actions_before_positions_threshold() {
        let columns = PositioningChangeColumns::for_width(610.0);

        assert!(columns.trader_width < POSITIONING_TRADER_COMPACT_ACTIONS_MIN_WIDTH);
        assert_eq!(
            positioning_trader_action_visibility(
                columns.trader_width,
                POSITIONING_CHANGE_TRADER_COMPACT_ACTIONS_MIN_WIDTH,
            ),
            (true, false)
        );
    }
}
