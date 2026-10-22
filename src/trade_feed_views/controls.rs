use super::{trade_notional_label, trade_uses_dollar_quote};
use crate::app_state::TradingTerminal;
use crate::config::OrderBookSymbolModeConfig;
use crate::helpers::design::{
    CONTROL_HEIGHT, CONTROL_TEXT_SIZE, META_TEXT_SIZE, UiColors, compact_button_style,
    compact_input_style, mix_color, readable_color,
};
use crate::message::Message;
use crate::trade_feed_state::{
    TradeFeedId, TradeFeedInstance, TradeFeedSource, parse_trade_feed_minimum,
    trade_feed_symbol_rank,
};
use iced::widget::{Column, Row, button, column, container, row, svg, text, text_input, tooltip};
use iced::{Alignment, Border, Color, Element, Fill};

const CHEVRON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="m4 6 4 4 4-4"/></svg>"#;

impl TradingTerminal {
    pub(super) fn view_trade_feed_controls<'a>(
        &'a self,
        id: TradeFeedId,
        feed: &'a TradeFeedInstance,
        quote: &str,
        width: f32,
        colors: UiColors,
    ) -> Element<'a, Message> {
        let requested = match &feed.mode {
            OrderBookSymbolModeConfig::Active => &self.active_symbol,
            OrderBookSymbolModeConfig::Fixed(key) => key,
        };
        let label = if requested.is_empty() {
            "Select asset".to_string()
        } else {
            self.display_name_for_symbol(requested)
        };
        let radius = self.pane_corner_radius;
        let picker_open = feed.picker_open;
        let asset = button(
            row![
                container(text(label.clone()).size(13).wrapping(text::Wrapping::None))
                    .width(Fill)
                    .clip(true),
                svg(svg::Handle::from_memory(CHEVRON))
                    .width(14)
                    .height(14)
                    .style(move |_, _| svg::Style {
                        color: Some(colors.muted)
                    }),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .height(CONTROL_HEIGHT)
        .width(Fill)
        .padding([0, 8])
        .on_press(Message::TradeFeedTogglePicker(id))
        .style(move |theme, status| compact_button_style(theme, status, picker_open, radius));
        let follow = control(
            "Follow active",
            Message::TradeFeedSetMode(id, OrderBookSymbolModeConfig::Active),
            matches!(feed.mode, OrderBookSymbolModeConfig::Active),
            radius,
        );
        let title = row![tip(asset.into(), label, colors, radius), follow]
            .spacing(4)
            .align_y(Alignment::Center);
        let grouping = tip(
            control(
                "Group trades",
                Message::TradeFeedToggleGrouping(id),
                feed.group_trades,
                radius,
            ),
            "Combine the same trader, side and action within one second; summed size and weighted average price.",
            colors,
            radius,
        );
        let context_hint = if self.hydromancer_api_key.trim().is_empty() {
            "Trade context requires a Hydromancer key in Settings > Integrations. Using Hyperliquid."
        } else if feed.failed_context_generation.is_some() {
            "Hydromancer context unavailable. Using Hyperliquid; Retry to reconnect."
        } else {
            "Show Open / Add / Reduce / Close / Flip using Hydromancer. Off uses Hyperliquid."
        };
        let mut options = row![
            grouping,
            tip(
                control(
                    "Context",
                    Message::TradeFeedToggleContext(id),
                    feed.show_context,
                    radius
                ),
                context_hint,
                colors,
                radius,
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center);
        if feed.show_context
            && feed.failed_context_generation.is_some()
            && !self.hydromancer_api_key.trim().is_empty()
        {
            options = options.push(control(
                "Retry",
                Message::TradeFeedRetryContext(id),
                false,
                radius,
            ));
        }
        let mut content = Column::new().spacing(4).width(Fill);
        if width >= 560.0 {
            content = content.push(
                row![title.width(Fill), options]
                    .spacing(8)
                    .align_y(Alignment::Center),
            );
        } else {
            content = content.push(title).push(options.wrap());
        }
        let (status, tone) = feed_status(feed, colors);
        let provider = match feed.source {
            TradeFeedSource::Hydromancer { .. } => "Hydromancer",
            TradeFeedSource::Hyperliquid => "Hyperliquid",
        };
        let mut status_row = row![
            badge(status, tone, colors, radius),
            text(provider).size(META_TEXT_SIZE).color(colors.muted),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if feed.has_gap {
            status_row = status_row.push(tip(
                badge("History gap", colors.warning, colors, radius),
                "Some trades were missed. The tape and summary include received trades only.",
                colors,
                radius,
            ));
        }
        if feed.is_held() {
            status_row = status_row.push(tip(
                badge("Rows held", colors.accent, colors, radius),
                "Rows stay still while wallet actions are open. New trades and summary totals continue updating.",
                colors,
                radius,
            ));
        }
        if feed.show_context && matches!(feed.source, TradeFeedSource::Hyperliquid) {
            status_row = status_row.push(tip(
                badge("No context", colors.warning, colors, radius),
                context_hint,
                colors,
                radius,
            ));
        }
        content = content.push(status_row.wrap().vertical_spacing(4));
        if feed.picker_open {
            content = content.push(self.view_trade_feed_picker(id, feed, colors));
        }
        if feed.group_trades {
            content = content.push(view_minimum(id, feed, quote, colors, radius));
        }
        container(content).padding([4, 8]).width(Fill).into()
    }

    fn view_trade_feed_picker<'a>(
        &'a self,
        id: TradeFeedId,
        feed: &'a TradeFeedInstance,
        colors: UiColors,
    ) -> Element<'a, Message> {
        let radius = self.pane_corner_radius;
        let search = text_input("Search asset…", &feed.search)
            .size(CONTROL_TEXT_SIZE)
            .line_height(text::LineHeight::Absolute(16.0.into()))
            .padding([6, 8])
            .on_input(move |query| Message::TradeFeedSearchChanged(id, query))
            .style(move |theme, status| compact_input_style(theme, status, false, radius));
        let query = feed.search.to_lowercase();
        let mut matches: Vec<_> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| {
                symbol.is_user_selectable_market() && !self.exchange_symbol_is_hidden(symbol)
            })
            .filter_map(|symbol| {
                let display = Self::exchange_symbol_display_name(symbol);
                trade_feed_symbol_rank(&query, &symbol.key, &display)
                    .map(|rank| (rank, symbol, display))
            })
            .collect();
        matches.sort_by(|a, b| (a.0, &a.2, &a.1.key).cmp(&(b.0, &b.2, &b.1.key)));
        let mut results = Column::new().spacing(4).width(Fill);
        if matches.is_empty() {
            results = results.push(
                container(text("No matching assets").size(12).color(colors.muted)).padding([4, 8]),
            );
        }
        for (_, symbol, display) in matches.into_iter().take(5) {
            let selected =
                matches!(&feed.mode, OrderBookSymbolModeConfig::Fixed(key) if key == &symbol.key);
            results = results.push(
                button(
                    container(
                        text(display)
                            .size(CONTROL_TEXT_SIZE)
                            .wrapping(text::Wrapping::None),
                    )
                    .width(Fill)
                    .clip(true),
                )
                .height(28)
                .width(Fill)
                .padding([0, 8])
                .on_press(Message::TradeFeedSetMode(
                    id,
                    OrderBookSymbolModeConfig::Fixed(symbol.key.clone()),
                ))
                .style(move |theme, status| compact_button_style(theme, status, selected, radius)),
            );
        }
        column![search, results].spacing(4).into()
    }
}

pub(super) fn control(
    label: &str,
    message: Message,
    selected: bool,
    radius: f32,
) -> Element<'static, Message> {
    button(
        text(label.to_owned())
            .size(CONTROL_TEXT_SIZE)
            .line_height(text::LineHeight::Absolute(16.0.into())),
    )
    .height(CONTROL_HEIGHT)
    .padding([0, 8])
    .on_press(message)
    .style(move |theme, status| compact_button_style(theme, status, selected, radius))
    .into()
}

pub(super) fn tip<'a>(
    content: Element<'a, Message>,
    label: impl Into<String>,
    colors: UiColors,
    radius: f32,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(label.into()).size(META_TEXT_SIZE).color(colors.text))
            .padding([6, 8])
            .max_width(300)
            .style(move |_| colors.overlay(radius)),
        tooltip::Position::Bottom,
    )
    .into()
}

fn badge(label: &str, tone: Color, colors: UiColors, radius: f32) -> Element<'static, Message> {
    let background = mix_color(colors.panel, tone, 0.12);
    container(
        text(label.to_owned())
            .size(META_TEXT_SIZE)
            .line_height(text::LineHeight::Absolute(16.0.into())),
    )
    .height(18)
    .padding([0, 6])
    .center_y(18)
    .style(move |_| container::Style {
        background: Some(background.into()),
        text_color: Some(readable_color(tone, background, 4.5)),
        border: Border {
            radius: crate::config::effective_radius(radius, 3.0).into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn feed_status(feed: &TradeFeedInstance, colors: UiColors) -> (&'static str, Color) {
    if feed.symbol.is_none() {
        return ("Asset unavailable", colors.negative);
    }
    match feed.status {
        "Live" => ("Live", colors.positive),
        "Waiting for trades" => ("Waiting", colors.muted),
        "Reconnecting" | "Catching up" => (feed.status, colors.warning),
        "Feed unavailable" => ("Unavailable", colors.negative),
        status => (status, colors.muted),
    }
}

fn view_minimum<'a>(
    id: TradeFeedId,
    feed: &'a TradeFeedInstance,
    quote: &str,
    colors: UiColors,
    radius: f32,
) -> Element<'a, Message> {
    let invalid = parse_trade_feed_minimum(&feed.min_group_notional_input).is_none();
    let unit = if trade_uses_dollar_quote(quote) {
        "$"
    } else {
        quote
    };
    let hint = format!(
        "Minimum combined notional per group. Use 10000, 10,000 or 10k; blank shows all. Current minimum: {}.",
        trade_notional_label(feed.min_group_notional, quote)
    );
    let input = text_input("0", &feed.min_group_notional_input)
        .size(CONTROL_TEXT_SIZE)
        .line_height(text::LineHeight::Absolute(16.0.into()))
        .padding([6, 8])
        .width(116)
        .on_input(move |value| Message::TradeFeedMinimumChanged(id, value))
        .style(move |theme, status| compact_input_style(theme, status, invalid, radius));
    let mut field = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(
            text(format!("Min {unit}"))
                .size(META_TEXT_SIZE)
                .color(colors.muted),
        )
        .push(tip(input.into(), hint, colors, radius));
    if !feed.min_group_notional_input.is_empty() {
        field = field.push(control(
            "Clear",
            Message::TradeFeedMinimumChanged(id, String::new()),
            false,
            radius,
        ));
    }
    let mut content = column![field.wrap()].spacing(4);
    if invalid {
        content = content.push(
            text("Enter a valid amount; the previous minimum is still applied.")
                .size(META_TEXT_SIZE)
                .color(colors.negative),
        );
    }
    content.into()
}
