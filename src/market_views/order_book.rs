use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookDisplayMode, OrderBookId, OrderBookSymbolMode};
use crate::message::Message;

use iced::widget::{column, container, row, rule, text};
use iced::{Element, Fill, color};

mod controls;
mod depth;
mod settings;
mod user_orders;

pub(super) use user_orders::UserOrderBookLevels;

impl TradingTerminal {
    pub(crate) fn view_order_book(&self, id: OrderBookId) -> Element<'_, Message> {
        let Some(inst) = self.order_books.get(&id) else {
            return container(text("Loading Order Book...").size(12).style(
                move |t: &iced::Theme| text::Style {
                    color: Some(t.extended_palette().background.weak.text),
                },
            ))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .into();
        };
        let theme = self.theme();
        let tracking_symbol = match &inst.mode {
            OrderBookSymbolMode::Active => self.active_symbol.clone(),
            OrderBookSymbolMode::Fixed(symbol) => symbol.clone(),
        };
        if !tracking_symbol.is_empty() && self.symbol_key_is_hidden(&tracking_symbol) {
            let content = column![
                text("Order Book").size(13).color(theme.palette().text),
                rule::horizontal(1),
                container(
                    text("Muted ticker")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text)
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            ]
            .spacing(8);
            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let mid = inst.book.mid_price();
        let tick_options = helpers::book_tick_options(mid);
        let tick = Self::resolved_order_book_tick(inst, &tick_options);
        let tick_buttons = Self::view_order_book_tick_buttons(id, &tick_options, tick);
        let header = match inst.display_mode {
            OrderBookDisplayMode::DepthList => Self::view_order_book_header(inst.reverse_side),
            OrderBookDisplayMode::DomLadder => Self::view_order_book_dom_header(inst.reverse_side),
        };
        let waiting_for_selected_precision = !inst.can_render_book_at_tick(tick);
        let title_row = self.view_order_book_title(id, inst);
        let outcome_metadata = self.view_order_book_outcome_metadata(&tracking_symbol, inst);

        if waiting_for_selected_precision
            || (inst.book.bids.is_empty() && inst.book.asks.is_empty())
        {
            let loading_row: Element<'_, Message> = if let Some(error) = &inst.book_error {
                column![
                    text("Order book unavailable")
                        .size(12)
                        .color(color!(0xff5555)),
                    text(error)
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(4)
                .into()
            } else if inst.book_loading {
                row![
                    self.view_spinner(18),
                    text(if waiting_for_selected_precision {
                        "Loading selected denomination..."
                    } else {
                        "Loading order book..."
                    })
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .into()
            } else {
                text(if waiting_for_selected_precision {
                    "Waiting for selected denomination"
                } else {
                    "No order book data"
                })
                .size(12)
                .color(theme.extended_palette().background.weak.text)
                .into()
            };
            let mut content = column![title_row].spacing(4);
            if let Some(outcome_metadata) = outcome_metadata {
                content = content.push(outcome_metadata);
            }
            content = content
                .push(tick_buttons)
                .push(header)
                .push(rule::horizontal(1))
                .push(loading_row);

            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let user_order_levels = self.user_order_book_levels(&tracking_symbol, tick);
        let scroll = match inst.display_mode {
            OrderBookDisplayMode::DepthList => {
                Self::view_order_book_rows(id, inst, tick, &theme, &user_order_levels)
            }
            OrderBookDisplayMode::DomLadder => {
                Self::view_order_book_dom_ladder(id, inst, tick, &theme, &user_order_levels)
            }
        };

        let mut content_col = column![title_row].spacing(4);
        if let Some(outcome_metadata) = outcome_metadata {
            content_col = content_col.push(outcome_metadata);
        }
        if let Some(error) = &inst.book_error {
            content_col = content_col.push(
                text(format!("{error}; showing last snapshot"))
                    .size(11)
                    .color(color!(0xff5555)),
            );
        }

        if inst.settings_open {
            content_col = content_col.push(self.view_order_book_settings(id, inst));
        }

        content_col = content_col
            .push(tick_buttons)
            .push(header)
            .push(rule::horizontal(1))
            .push(scroll);

        if inst.show_spread_chart {
            content_col = content_col
                .push(rule::horizontal(1))
                .push(Self::view_order_book_spread_chart(id, inst));
        }

        container(content_col)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn user_order_book_levels(&self, symbol: &str, tick: f64) -> UserOrderBookLevels {
        let orders = self.merged_open_orders();
        UserOrderBookLevels::from_orders(&orders, symbol, tick)
    }
}

#[cfg(test)]
mod tests;
