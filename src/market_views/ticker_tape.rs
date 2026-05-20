use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, float, responsive, row, rule, stack, text};
use iced::{Color, Element, Fill, Length, Theme, Vector};

// ---------------------------------------------------------------------------
// Ticker Tape
// ---------------------------------------------------------------------------

const TICKER_TAPE_HEIGHT: f32 = 34.0;
const TICKER_TAPE_ICON_SIZE: f32 = 16.0;
const TICKER_TAPE_ITEM_MIN_WIDTH: f32 = 148.0;
const TICKER_TAPE_ITEM_MAX_WIDTH: f32 = 240.0;
const TICKER_TAPE_ITEM_HORIZONTAL_PADDING: u16 = 12;
const TICKER_TAPE_ITEM_SPACING: u32 = 5;
const TICKER_TAPE_SEPARATOR_WIDTH: f32 = 1.0;
const TICKER_TAPE_TEXT_CHAR_WIDTH: f32 = 7.0;

#[derive(Debug, Clone)]
struct TickerTapeItem {
    symbol: String,
    ticker: String,
    price: Option<f64>,
    pct_24h: Option<f64>,
}

impl TradingTerminal {
    pub(crate) fn ticker_tape_bar_height(&self) -> f32 {
        if self.ticker_tape_enabled {
            TICKER_TAPE_HEIGHT
        } else {
            0.0
        }
    }

    pub(crate) fn view_ticker_tape_bar(&self) -> Element<'_, Message> {
        responsive(|size| self.view_ticker_tape_bar_sized(size.width))
            .height(Length::Fixed(TICKER_TAPE_HEIGHT))
            .into()
    }

    fn view_ticker_tape_bar_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let items = self.ticker_tape_items();
        let denomination = self.display_denomination_context();
        let pane_corner_radius = self.pane_corner_radius;

        if items.is_empty() {
            return container(
                text("No favourites")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Length::Fixed(TICKER_TAPE_HEIGHT))
            .padding([0, 12])
            .center_y(Length::Fixed(TICKER_TAPE_HEIGHT))
            .style(move |theme: &Theme| ticker_tape_bar_style(theme, pane_corner_radius))
            .into();
        }

        let item_widths: Vec<f32> = items
            .iter()
            .map(|item| ticker_tape_item_width(item, &denomination))
            .collect();
        let sequence_width: f32 = item_widths
            .iter()
            .map(|width| width + TICKER_TAPE_SEPARATOR_WIDTH)
            .sum();
        let should_scroll = sequence_width > available_width.max(0.0);
        let offset = if should_scroll {
            self.ticker_tape_scroll_px.rem_euclid(sequence_width)
        } else {
            0.0
        };
        let repetitions = if should_scroll { 2 } else { 1 };

        let mut tape_row = row![]
            .spacing(0)
            .height(Length::Fixed(TICKER_TAPE_HEIGHT))
            .align_y(iced::Alignment::Center);
        for _ in 0..repetitions {
            for (item, item_width) in items.iter().zip(item_widths.iter().copied()) {
                tape_row = tape_row
                    .push(ticker_tape_item(item, &denomination, &theme, item_width))
                    .push(ticker_tape_separator());
            }
        }

        let tape_width = sequence_width * repetitions as f32;
        let tape = container(tape_row)
            .width(Length::Fixed(tape_width))
            .center_y(Length::Fixed(TICKER_TAPE_HEIGHT));

        let tape_layer: Element<'_, Message> = if should_scroll {
            float(tape)
                .translate(move |_bounds, _viewport| Vector::new(-offset, 0.0))
                .into()
        } else {
            tape.into()
        };

        let layers: Vec<Element<'_, Message>> = vec![
            Space::new()
                .width(Fill)
                .height(Length::Fixed(TICKER_TAPE_HEIGHT))
                .into(),
            tape_layer,
        ];

        container(
            stack(layers)
                .width(Fill)
                .height(Length::Fixed(TICKER_TAPE_HEIGHT))
                .clip(true),
        )
        .width(Fill)
        .height(Length::Fixed(TICKER_TAPE_HEIGHT))
        .clip(true)
        .style(move |theme: &Theme| ticker_tape_bar_style(theme, pane_corner_radius))
        .into()
    }

    fn ticker_tape_items(&self) -> Vec<TickerTapeItem> {
        self.favourite_symbols
            .iter()
            .filter(|symbol| !self.symbol_key_is_hidden(symbol))
            .filter_map(|symbol| self.ticker_tape_item(symbol))
            .collect()
    }

    fn ticker_tape_item(&self, symbol: &str) -> Option<TickerTapeItem> {
        let symbol_meta = self.resolve_exchange_symbol_by_key_or_ticker(symbol);
        if symbol_meta.is_some_and(|symbol| !symbol.is_user_selectable_market()) {
            return None;
        }

        let ticker = symbol_meta
            .map(|symbol| symbol.ticker.clone())
            .unwrap_or_else(|| symbol.split(':').next_back().unwrap_or(symbol).to_string());
        let price = self.resolve_mid_for_symbol(symbol);
        let ctx = self
            .ticker_tape_ctxs
            .get(symbol)
            .or_else(|| symbol_meta.and_then(|symbol| self.ticker_tape_ctxs.get(&symbol.ticker)));
        let pct_24h = ctx.and_then(|ctx| percent_change(price, ctx.prev_day_px));

        Some(TickerTapeItem {
            symbol: symbol.to_string(),
            ticker,
            price,
            pct_24h,
        })
    }
}

fn ticker_tape_item(
    item: &TickerTapeItem,
    denomination: &crate::denomination::DisplayDenominationContext,
    theme: &Theme,
    item_width: f32,
) -> Element<'static, Message> {
    let icon = helpers::symbol_icon(
        &item.symbol,
        TICKER_TAPE_ICON_SIZE as u16,
        theme.palette().text,
    )
    .map(Element::from)
    .unwrap_or_else(|| fallback_ticker_logo(&item.ticker, theme));
    let pct_color = pct_color(item.pct_24h, theme);
    let symbol = item.symbol.clone();

    let content = row![
        icon,
        text(item.ticker.clone())
            .size(12)
            .font(iced::Font::MONOSPACE),
        text(price_label(item.price, denomination))
            .size(12)
            .font(iced::Font::MONOSPACE),
        text(percent_label(item.pct_24h))
            .size(12)
            .font(iced::Font::MONOSPACE)
            .color(pct_color),
    ]
    .spacing(TICKER_TAPE_ITEM_SPACING)
    .align_y(iced::Alignment::Center);

    let centered_content = container(content)
        .width(Fill)
        .height(Length::Fixed(TICKER_TAPE_HEIGHT))
        .center_y(Length::Fixed(TICKER_TAPE_HEIGHT));

    container(
        button(centered_content)
            .on_press(Message::SymbolSelected(symbol))
            .width(Fill)
            .height(Length::Fixed(TICKER_TAPE_HEIGHT))
            .padding([0, TICKER_TAPE_ITEM_HORIZONTAL_PADDING])
            .style(|theme: &Theme, status| ticker_tape_item_button_style(theme, status)),
    )
    .width(Length::Fixed(item_width))
    .center_y(Length::Fixed(TICKER_TAPE_HEIGHT))
    .clip(true)
    .into()
}

fn ticker_tape_item_width(
    item: &TickerTapeItem,
    denomination: &crate::denomination::DisplayDenominationContext,
) -> f32 {
    let text_chars = item.ticker.chars().count()
        + price_label(item.price, denomination).chars().count()
        + percent_label(item.pct_24h).chars().count();
    let text_width = text_chars as f32 * TICKER_TAPE_TEXT_CHAR_WIDTH;
    let padding = f32::from(TICKER_TAPE_ITEM_HORIZONTAL_PADDING) * 2.0;
    let spacing = TICKER_TAPE_ITEM_SPACING as f32 * 3.0;
    let width = TICKER_TAPE_ICON_SIZE + text_width + padding + spacing;

    width
        .ceil()
        .clamp(TICKER_TAPE_ITEM_MIN_WIDTH, TICKER_TAPE_ITEM_MAX_WIDTH)
}

fn ticker_tape_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.10,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .width(Length::Fixed(TICKER_TAPE_SEPARATOR_WIDTH))
    .height(18)
    .center_y(Length::Fixed(TICKER_TAPE_HEIGHT))
    .into()
}

fn fallback_ticker_logo(ticker: &str, theme: &Theme) -> Element<'static, Message> {
    let label = ticker
        .chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());
    let color = theme.palette().primary;

    container(
        text(label)
            .size(9)
            .font(iced::Font::MONOSPACE)
            .color(color)
            .center(),
    )
    .center_x(Length::Fixed(TICKER_TAPE_ICON_SIZE))
    .center_y(Length::Fixed(TICKER_TAPE_ICON_SIZE))
    .style(move |_theme: &Theme| {
        let mut background = color;
        background.a = 0.12;

        container_style::Style {
            background: Some(background.into()),
            border: iced::Border {
                radius: (TICKER_TAPE_ICON_SIZE * 0.5).into(),
                width: 1.0,
                color,
            },
            ..Default::default()
        }
    })
    .into()
}

fn price_label(
    price: Option<f64>,
    denomination: &crate::denomination::DisplayDenominationContext,
) -> String {
    price
        .map(|price| denomination.format_price(price))
        .unwrap_or_else(|| "-".to_string())
}

fn percent_label(pct: Option<f64>) -> String {
    pct.map(|pct| format!("{pct:+.2}%"))
        .unwrap_or_else(|| "-".to_string())
}

fn percent_change(current: Option<f64>, previous: Option<f64>) -> Option<f64> {
    let current = current?;
    let previous = previous?;
    if current > 0.0 && previous > 0.0 {
        Some((current - previous) / previous * 100.0)
    } else {
        None
    }
}

fn pct_color(pct: Option<f64>, theme: &Theme) -> Color {
    match pct {
        Some(value) if value >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.extended_palette().background.weak.text,
    }
}

fn ticker_tape_bar_style(theme: &Theme, corner_radius: f32) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        text_color: Some(theme.palette().text),
        border: iced::Border {
            width: 1.0,
            color: border_color,
            radius: corner_radius.into(),
        },
        ..Default::default()
    }
}

fn ticker_tape_item_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(theme.extended_palette().background.weak.color.into())
        }
        _ => None,
    };

    button::Style {
        background,
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 3.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}
