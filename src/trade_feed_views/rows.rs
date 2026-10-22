use super::colors::TraderColors;
use super::controls::tip;
use super::layout::{
    CELL_INSET, CONTEXT_WIDTH, GROUP_COUNT_WIDTH, ROW_HEIGHT, SIDE_WIDTH, TIME_WIDTH, TRADER_WIDTH,
    TapeRowLayout,
};
use super::{trade_age_label, trade_intent_label, trade_notional_label, trade_value_label};
use crate::app_state::TradingTerminal;
use crate::helpers::design::{UiColors, compact_button_style, readable_color};
use crate::message::Message;
use crate::trade_feed_state::{TradeFeedId, TradeFeedInstance, TradePrint, trade_shading};
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};
use iced::widget::{Column, button, container, row, text};
use iced::{Alignment, Color, Element, Fill, Length};

struct TapeCells {
    time: Element<'static, Message>,
    side: Element<'static, Message>,
    trader: Element<'static, Message>,
    context: Element<'static, Message>,
    price: Element<'static, Message>,
    size: Element<'static, Message>,
    value: Element<'static, Message>,
}

pub(super) fn view_header(
    id: TradeFeedId,
    feed: &TradeFeedInstance,
    layout: TapeRowLayout,
    colors: UiColors,
    radius: f32,
) -> Element<'static, Message> {
    let label = |value: &str, width| {
        container(text(value.to_owned()).size(11).color(colors.muted))
            .width(width)
            .into()
    };
    tape_row(
        layout,
        TapeCells {
            time: label("Time", Length::Fixed(TIME_WIDTH)),
            side: label("Side", Length::Fixed(SIDE_WIDTH)),
            trader: label("Trader", Length::Fixed(TRADER_WIDTH)),
            context: label("Action", Length::Fixed(CONTEXT_WIDTH)),
            price: numeric_cell(
                if feed.group_trades {
                    "Avg price"
                } else {
                    "Price"
                },
                Some(colors.muted),
                11,
                false,
            ),
            size: size_cell(
                id,
                if feed.size_in_notional {
                    "Notional"
                } else {
                    "Size"
                },
                feed.size_in_notional,
                colors.muted,
                colors,
                radius,
                true,
            ),
            value: numeric_cell("Value", Some(colors.muted), 11, false),
        },
    )
}

impl TradingTerminal {
    pub(super) fn view_trade_feed_rows(
        &self,
        id: TradeFeedId,
        feed: &TradeFeedInstance,
        layout: TapeRowLayout,
        height: f32,
        quote: &str,
    ) -> Element<'static, Message> {
        let theme = self.theme();
        let colors = UiColors::new(&theme);
        let traders = TraderColors::new(&theme);
        let rows = feed.visible_rows();
        if rows.is_empty() {
            return container(text(empty_message(feed)).size(12).color(colors.muted))
                .padding(8)
                .center(Fill)
                .into();
        }
        let largest = rows.iter().map(|item| item.value).fold(0.0, f64::max);
        let mut content = Column::new().spacing(0).width(Fill);
        for item in rows.iter().take(TapeRowLayout::row_count(height)) {
            let trade = item.latest;
            let background = super::colors::row_background(
                colors,
                if trade.is_buy {
                    theme.palette().success
                } else {
                    theme.palette().danger
                },
                trade_shading(item.value, largest),
            );
            let foreground = readable_color(colors.text, background, 4.5);
            let muted = readable_color(colors.muted, background, 4.5);
            let side = readable_color(
                if trade.is_buy {
                    colors.positive
                } else {
                    colors.negative
                },
                background,
                4.5,
            );
            let mut trader = row![self.view_trade_feed_trader(id, feed, trade, &traders, colors)]
                .align_y(Alignment::Center)
                .width(TRADER_WIDTH);
            if feed.group_trades {
                trader = trader.push(
                    container(text(format!("×{}", item.count)).size(11).color(muted))
                        .align_right(GROUP_COUNT_WIDTH)
                        .clip(true),
                );
            }
            let size = if feed.size_in_notional {
                trade_notional_label(item.value, quote)
            } else {
                item.size_label().into_owned()
            };
            let price = item.price_label();
            let value = trade_value_label(item.value);
            let cells = TapeCells {
                time: container(
                    text(trade_age_label(trade.time_ms, self.status_bar_now_ms))
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(muted)
                        .wrapping(text::Wrapping::None),
                )
                .width(TIME_WIDTH)
                .clip(true)
                .into(),
                side: container(
                    text(if trade.is_buy { "Buy" } else { "Sell" })
                        .size(11)
                        .color(side),
                )
                .width(SIDE_WIDTH)
                .into(),
                trader: trader.into(),
                context: container(
                    text(trade_intent_label(trade.intent))
                        .size(11)
                        .color(muted)
                        .wrapping(text::Wrapping::None),
                )
                .width(CONTEXT_WIDTH)
                .clip(true)
                .into(),
                price: tip(
                    numeric_cell(&price, Some(foreground), 12, true),
                    price.to_string(),
                    colors,
                    self.pane_corner_radius,
                ),
                size: size_cell(
                    id,
                    &size,
                    feed.size_in_notional,
                    foreground,
                    colors,
                    self.pane_corner_radius,
                    false,
                ),
                value: tip(
                    numeric_cell(&value, Some(foreground), 12, true),
                    trade_notional_label(item.value, quote),
                    colors,
                    self.pane_corner_radius,
                ),
            };
            content =
                content.push(
                    container(tape_row(layout, cells)).style(move |_| container::Style {
                        background: Some(background.into()),
                        ..Default::default()
                    }),
                );
        }
        content.into()
    }

    fn view_trade_feed_trader(
        &self,
        id: TradeFeedId,
        feed: &TradeFeedInstance,
        trade: &TradePrint,
        traders: &TraderColors,
        colors: UiColors,
    ) -> Element<'static, Message> {
        let width = TRADER_WIDTH
            - if feed.group_trades {
                GROUP_COUNT_WIDTH
            } else {
                0.0
            };
        let Some(address) = &trade.trader else {
            return container(text("—").size(12).color(colors.muted))
                .width(width)
                .into();
        };
        let mut display = self.wallet_display(address.as_str());
        if feed.group_trades && !display.has_label {
            // Leave room for the count without clipping the identifying suffix.
            let address = address.as_str();
            if let (Some(prefix), Some(suffix)) = (
                address.get(..4),
                address.get(address.len().saturating_sub(4)..),
            ) {
                display.primary = format!("{prefix}…{suffix}");
            }
        }
        let tooltip_label = if display.has_label {
            format!("{} ({})", display.primary, address.as_str())
        } else {
            format!("Copy {}", address.as_str())
        };
        container(wallet_address_action_cell(WalletAddressActionCell {
            address: address.as_str().to_owned(),
            label: display.primary,
            tooltip_label,
            hover_key: feed.address_hover_key(id, trade),
            hovered_key: self.hovered_wallet_address_actions.as_deref(),
            width,
            text_size: 12,
            text_color: traders.for_address(address.as_str()),
        }))
        .width(width)
        .clip(true)
        .into()
    }
}

fn tape_row(layout: TapeRowLayout, cells: TapeCells) -> Element<'static, Message> {
    let mut row = row![cells.side, cells.trader]
        .spacing(layout.gap)
        .align_y(Alignment::Center);
    if layout.show_context {
        row = row.push(cells.context);
    }
    if layout.show_price {
        row = row.push(cells.price);
    }
    row = row.push(cells.size);
    if layout.show_value {
        row = row.push(cells.value);
    }
    if layout.show_time {
        row = row.push(cells.time);
    }
    container(row)
        .height(ROW_HEIGHT)
        .padding([0.0, CELL_INSET])
        .center_y(ROW_HEIGHT)
        .width(Fill)
        .clip(true)
        .into()
}

fn numeric_cell(
    value: &str,
    color: Option<Color>,
    size: u32,
    numeric: bool,
) -> Element<'static, Message> {
    let value = text(value.to_owned())
        .size(size)
        .color_maybe(color)
        .wrapping(text::Wrapping::None);
    let value = if numeric {
        value.font(crate::app_fonts::monospace_font())
    } else {
        value
    };
    container(value)
        .align_right(Length::FillPortion(3))
        .clip(true)
        .into()
}

fn size_cell(
    id: TradeFeedId,
    label: &str,
    notional: bool,
    color: Color,
    colors: UiColors,
    radius: f32,
    header: bool,
) -> Element<'static, Message> {
    let hint = if notional {
        "Click to show asset quantity"
    } else {
        "Click to show notional value"
    };
    // Inherit the button foreground so hover surfaces get the same contrast
    // correction as other compact controls instead of a fixed text override.
    let content = numeric_cell(label, None, if header { 11 } else { 12 }, !header);
    tip(
        button(content)
            .padding(0)
            .width(Length::FillPortion(2))
            .height(ROW_HEIGHT)
            .on_press(Message::TradeFeedToggleSizeUnit(id))
            .style(move |theme, status| {
                let mut style = compact_button_style(theme, status, false, 0.0);
                if style.background.is_none() {
                    style.text_color = color;
                }
                style
            })
            .into(),
        format!("{label} · {hint}"),
        colors,
        radius,
    )
}

fn empty_message(feed: &TradeFeedInstance) -> &'static str {
    if feed.symbol.is_none() {
        "Select an available asset to see trades"
    } else if feed.status == "Feed unavailable" {
        "Trade feed unavailable"
    } else if feed.status == "Reconnecting" {
        "Reconnecting to the trade feed…"
    } else if !feed.visible_trades().is_empty()
        && feed.group_trades
        && feed.min_group_notional > 0.0
    {
        "No groups meet the minimum"
    } else {
        "Waiting for trades…"
    }
}
