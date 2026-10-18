use crate::app_state::TradingTerminal;
use crate::config::OrderBookSymbolModeConfig;
use crate::helpers;
use crate::message::Message;
use crate::trade_feed_state::{TradeFeedId, trade_feed_symbol_rank, trade_shading};
use iced::widget::{Column, button, column, container, responsive, row, rule, text, text_input};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(crate) fn view_trade_feed(&self, id: TradeFeedId) -> Element<'_, Message> {
        let Some(feed) = self.trade_feeds.instances.get(&id) else {
            return container(text("Loading Trade Feed...").size(12))
                .center(Fill)
                .into();
        };
        container(responsive(move |size| {
            let requested = match &feed.mode {
                OrderBookSymbolModeConfig::Active => &self.active_symbol,
                OrderBookSymbolModeConfig::Fixed(key) => key,
            };
            let label = if requested.is_empty() {
                "Select asset".to_string()
            } else {
                self.display_name_for_symbol(requested)
            };
            let follows = matches!(feed.mode, OrderBookSymbolModeConfig::Active);
            let theme = self.theme();
            let muted = theme.extended_palette().background.weak.text;
            let title = row![
                button(text(label).size(13).wrapping(text::Wrapping::None))
                    .width(Fill)
                    .on_press(Message::TradeFeedTogglePicker(id))
                    .style(button::text),
                button(text("Follow active").size(11))
                    .on_press(Message::TradeFeedSetMode(
                        id,
                        OrderBookSymbolModeConfig::Active
                    ))
                    .style(if follows {
                        button::primary
                    } else {
                        button::text
                    }),
            ]
            .spacing(4)
            .align_y(Alignment::Center);
            let mut content = column![title].spacing(4);
            if feed.picker_open {
                content = content.push(
                    text_input("Search asset...", &feed.search)
                        .size(12)
                        .padding(5)
                        .on_input(move |query| Message::TradeFeedSearchChanged(id, query)),
                );
                let query = feed.search.to_lowercase();
                let mut results = Column::new().spacing(2);
                let mut matches: Vec<_> = self
                    .exchange_symbols
                    .iter()
                    .filter(|symbol| {
                        symbol.is_user_selectable_market()
                            && !self.exchange_symbol_is_hidden(symbol)
                    })
                    .filter_map(|symbol| {
                        let display = Self::exchange_symbol_display_name(symbol);
                        trade_feed_symbol_rank(&query, &symbol.key, &display)
                            .map(|rank| (rank, symbol, display))
                    })
                    .collect();
                matches.sort_by(|a, b| (a.0, &a.2, &a.1.key).cmp(&(b.0, &b.2, &b.1.key)));
                for (_, symbol, display) in matches.into_iter().take(5) {
                    results = results.push(
                        button(text(display).size(12).wrapping(text::Wrapping::None))
                            .height(24)
                            .width(Fill)
                            .on_press(Message::TradeFeedSetMode(
                                id,
                                OrderBookSymbolModeConfig::Fixed(symbol.key.clone()),
                            ))
                            .style(button::text),
                    );
                }
                content = content.push(container(results).height(140).clip(true));
            }
            let status = if feed.symbol.is_none() {
                "Asset unavailable"
            } else {
                feed.status
            };
            let status = if feed.has_gap {
                format!("{status} · gap in history")
            } else {
                status.to_string()
            };
            content = content.push(text(status).size(11).color(muted));
            let wide = size.width >= 420.0;
            let header = tape_row("Time (UTC)", "Price", "Size", "Value", wide, muted);
            content = content.push(header).push(rule::horizontal(1));
            let available = size.height - if feed.picker_open { 255.0 } else { 80.0 };
            let row_count = ((available.max(0.0) / 24.0) as usize).min(100);
            let largest = feed
                .trades
                .iter()
                .map(|trade| trade.value)
                .fold(0.0, f64::max);
            for trade in feed.trades.iter().take(row_count) {
                let seconds = trade.time_ms / 1000;
                let time = format!(
                    "{:02}:{:02}:{:02}",
                    seconds / 3600 % 24,
                    seconds / 60 % 60,
                    seconds % 60
                );
                let value = helpers::format_size(trade.value);
                let accent = if trade.is_buy {
                    theme.palette().success
                } else {
                    theme.palette().danger
                };
                let shading = trade_shading(trade.value, largest);
                content = content.push(
                    container(tape_row(
                        &time,
                        &trade.price,
                        &trade.size,
                        &value,
                        wide,
                        theme.palette().text,
                    ))
                    .height(20)
                    .padding([1, 4])
                    .style(move |_theme: &Theme| container::Style {
                        background: Some(
                            Color {
                                a: shading,
                                ..accent
                            }
                            .into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 0.0,
                            radius: 2.0.into(),
                        },
                        ..Default::default()
                    }),
                );
            }
            content.width(Fill).into()
        }))
        .padding(8)
        .width(Fill)
        .height(Fill)
        .into()
    }
}

fn tape_row(
    time: &str,
    price: &str,
    size: &str,
    value: &str,
    wide: bool,
    color: Color,
) -> Element<'static, Message> {
    let cell = |value: &str, portions, right| {
        let text = text(value.to_owned())
            .size(12)
            .wrapping(text::Wrapping::None)
            .color(color)
            .width(Length::FillPortion(portions));
        if right {
            text.align_x(Alignment::End)
        } else {
            text
        }
    };
    let mut cells = row![
        cell(time, 2, false),
        cell(price, 3, true),
        cell(size, 3, true)
    ]
    .spacing(8);
    if wide {
        cells = cells.push(cell(value, 3, true));
    }
    cells.align_y(Alignment::Center).into()
}
