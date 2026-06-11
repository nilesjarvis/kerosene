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
            // Keep the real title row (and settings panel) so the user can
            // still rebind the widget to another symbol from here.
            let mut content =
                column![self.view_order_book_title(id, inst), rule::horizontal(1)].spacing(8);
            if inst.settings_open {
                content = content.push(self.view_order_book_settings(id, inst));
            }
            content = content.push(
                container(
                    text("Muted ticker")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            );
            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let tick_options = helpers::book_tick_options(inst.tick_options_mid());
        let tick = Self::resolved_order_book_tick(inst, &tick_options);
        let tick_buttons = Self::view_order_book_tick_buttons(id, &tick_options, tick);
        let header = match inst.display_mode {
            OrderBookDisplayMode::DepthList => {
                Some(Self::view_order_book_header(inst.reverse_side))
            }
            OrderBookDisplayMode::DomLadder => {
                Some(Self::view_order_book_dom_header(inst.reverse_side))
            }
            // The depth chart draws its own axis labels on the canvas.
            OrderBookDisplayMode::DepthChart => None,
        };
        let waiting_for_selected_precision = !inst.can_render_book_at_tick(tick);
        // While a finer denomination is being fetched, keep showing the book
        // at the freshest renderable granularity instead of blanking the
        // widget; the title-row spinner signals the fetch in flight.
        let render_tick = if waiting_for_selected_precision {
            inst.book_source_tick_size().unwrap_or(tick)
        } else {
            tick
        };
        let title_row = self.view_order_book_title(id, inst);
        let outcome_metadata = self.view_order_book_outcome_metadata(&tracking_symbol, inst);

        if inst.book.bids.is_empty() && inst.book.asks.is_empty() {
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
            if inst.settings_open {
                content = content.push(self.view_order_book_settings(id, inst));
            }
            content = content.push(tick_buttons);
            if let Some(header) = header {
                content = content.push(header);
            }
            content = content.push(rule::horizontal(1)).push(loading_row);

            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let user_order_levels = self.user_order_book_levels(&tracking_symbol, render_tick);
        let scroll = match inst.display_mode {
            OrderBookDisplayMode::DepthList => {
                Self::view_order_book_rows(id, inst, render_tick, &theme, &user_order_levels)
            }
            OrderBookDisplayMode::DomLadder => {
                Self::view_order_book_dom_ladder(id, inst, render_tick, &theme, &user_order_levels)
            }
            OrderBookDisplayMode::DepthChart => {
                Self::view_order_book_depth_chart(id, inst, render_tick, &user_order_levels)
            }
        };

        let mut content_col = column![title_row].spacing(4);
        if let Some(outcome_metadata) = outcome_metadata {
            content_col = content_col.push(outcome_metadata);
        }

        if inst.settings_open {
            content_col = content_col.push(self.view_order_book_settings(id, inst));
        }

        content_col = content_col.push(tick_buttons);
        if let Some(header) = header {
            content_col = content_col.push(header);
        }
        content_col = content_col.push(rule::horizontal(1)).push(scroll);

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
        self.account_data
            .as_ref()
            .map(|data| UserOrderBookLevels::from_orders(&data.open_orders, symbol, tick))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
