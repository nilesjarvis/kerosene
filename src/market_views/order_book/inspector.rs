use crate::app_state::TradingTerminal;
use crate::helpers::{
    format_decimal_with_commas, format_size, format_timestamp_exact, tick_decimals,
};
use crate::market_state::book_inspector::{PAGE_SIZE, SCALE, format_decimal};
use crate::message::Message;
use iced::widget::{
    Column, Space, button, column, container, row, rule, scrollable, text, tooltip,
};
use iced::{Element, Fill, Length};

impl TradingTerminal {
    pub(crate) fn view_book_inspector(&self) -> Element<'_, Message> {
        let Some(state) = &self.book_inspector else {
            return container(text("Order details closed")).into();
        };
        let theme = self.theme();
        let muted = theme.extended_palette().background.weak.text;
        let selection = state.selection;
        let decimals = tick_decimals(selection.tick);
        let price = format_decimal_with_commas(selection.price, decimals);
        let side = match selection.is_bid {
            Some(true) => "Bids",
            Some(false) => "Asks",
            None => "Orders",
        };
        let title = row![
            text(format!(
                "{} · {side} at {price}",
                self.display_name_for_symbol(&state.coin)
            ))
            .size(13),
            Space::new().width(Fill),
            button(text("Retry").size(11))
                .padding([3, 8])
                .on_press_maybe(
                    state
                        .status
                        .as_ref()
                        .filter(|status| status.can_retry())
                        .map(|_| Message::RetryBookInspector(state.window_id))
                ),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let range = match selection.is_bid {
            Some(true) => format!(
                "{price} ≤ price < {}",
                format_decimal_with_commas(selection.price + selection.tick, decimals)
            ),
            Some(false) => format!(
                "{} < price ≤ {price}",
                format_decimal_with_commas(selection.price - selection.tick, decimals)
            ),
            None => format!(
                "Bucket {price} · tick {}",
                format_decimal_with_commas(selection.tick, decimals)
            ),
        };
        let mut content = column![
            title,
            text(range).size(11).color(muted),
            rule::horizontal(1)
        ]
        .spacing(8);
        let unavailable = if self.symbol_key_is_hidden(&state.coin) {
            Some("This market is hidden in Settings > Risk.")
        } else if self.hydromancer_api_key.trim().is_empty() {
            Some("Add a Hydromancer API key with L4 access in Settings > Integrations.")
        } else {
            None
        };
        if let Some(message) = unavailable {
            return container(content.push(text(message).size(12)))
                .padding(12)
                .width(Fill)
                .height(Fill)
                .into();
        }
        if let Some(status) = &state.status {
            content = content.push(text(status.label()).size(11).color(muted));
        }
        if let Some(page) = &state.page {
            content = content.push(
                text(format!(
                    "Wallets: {} · Orders: {} · Size: {} · Notional: {} (quote)",
                    page.wallets,
                    page.total_orders,
                    format_size(page.total_size),
                    format_decimal_with_commas(page.notional, 2)
                ))
                .size(11),
            );
            let header = row![
                cell("Wallet", 3),
                cell("Side", 1),
                cell("Price", 2),
                cell("Size", 2),
                cell("Notional", 2),
                cell("Order ID", 3)
            ]
            .spacing(8);
            content = content.push(header).push(rule::horizontal(1));
            if page.orders.is_empty() {
                content = content.push(
                    container(
                        text("No resting orders in this block.")
                            .size(12)
                            .color(muted),
                    )
                    .width(Fill)
                    .height(Fill)
                    .center(Fill),
                );
            } else {
                let rows = page
                    .orders
                    .iter()
                    .fold(Column::new().spacing(2), |rows, order| {
                        let display = self.wallet_display(order.wallet.as_str());
                        let wallet_label = if display.has_label {
                            format!("{} · {}", display.primary, display.secondary)
                        } else {
                            display.primary
                        };
                        let wallet: Element<'_, Message> = tooltip(
                            button(
                                text(wallet_label)
                                    .size(11)
                                    .wrapping(iced::widget::text::Wrapping::None),
                            )
                            .style(button::text)
                            .padding([3, 0])
                            .width(Fill)
                            .on_press(Message::OpenWalletDetailsWindow(order.wallet.clone())),
                            text(order.wallet.as_str()).size(11),
                            tooltip::Position::Top,
                        )
                        .into();
                        let side_color = if order.is_bid {
                            theme.palette().success
                        } else {
                            theme.palette().danger
                        };
                        let notional = (order.price as f64 / SCALE as f64)
                            * (order.size as f64 / SCALE as f64);
                        rows.push(
                            row![
                                container(wallet).width(Length::FillPortion(3)).clip(true),
                                text(if order.is_bid { "Bid" } else { "Ask" })
                                    .size(11)
                                    .color(side_color)
                                    .width(Length::FillPortion(1)),
                                cell(format_decimal(order.price), 2),
                                cell(format_decimal(order.size), 2),
                                cell(format_decimal_with_commas(notional, 2), 2),
                                cell(order.oid.to_string(), 3),
                            ]
                            .spacing(8)
                            .align_y(iced::Alignment::Center),
                        )
                    });
                content = content.push(scrollable(rows).width(Fill).height(Fill));
            }
            let pages = page.total_orders.div_ceil(PAGE_SIZE).max(1);
            let paging = row![
                button(text("‹").size(12)).padding([3, 8]).on_press_maybe(
                    (page.page > 0)
                        .then(|| Message::BookInspectorPage(state.window_id, page.page - 1))
                ),
                text(format!("{} / {pages}", page.page + 1)).size(11),
                button(text("›").size(12)).padding([3, 8]).on_press_maybe(
                    (page.page + 1 < pages)
                        .then_some(Message::BookInspectorPage(state.window_id, page.page + 1))
                ),
                Space::new().width(Fill),
                text(format!(
                    "{} · {} · block {}",
                    if state.status.is_some() {
                        "Stale"
                    } else {
                        "Live"
                    },
                    format_timestamp_exact(page.timestamp_ms),
                    page.height
                ))
                .size(11)
                .color(muted),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);
            content = content.push(paging);
        } else {
            content = content.push(
                container(
                    text(
                        if state
                            .status
                            .as_ref()
                            .is_some_and(|status| status.can_retry())
                        {
                            "Order data unavailable"
                        } else {
                            "Loading orders…"
                        },
                    )
                    .size(12)
                    .color(muted),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            );
        }
        content = content.push(tooltip(
            text("Resting orders · Hydromancer").size(11).color(muted),
            text("Untriggered stop-loss and take-profit orders are not on the resting book.")
                .size(11),
            tooltip::Position::Top,
        ));
        container(content)
            .padding(12)
            .width(Fill)
            .height(Fill)
            .into()
    }
}

fn cell<'a>(value: impl Into<std::borrow::Cow<'a, str>>, width: u16) -> Element<'a, Message> {
    text(value.into())
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .width(Length::FillPortion(width))
        .wrapping(iced::widget::text::Wrapping::None)
        .into()
}
