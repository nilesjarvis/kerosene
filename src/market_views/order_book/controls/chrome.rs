use crate::app_state::TradingTerminal;
use crate::config::effective_radius;
use crate::helpers;
use crate::helpers::design::{UiColors, compact_button_style, mix_color, readable_color};
use crate::helpers::icons::{self, IconColor};
use crate::market_state::{
    OrderBookDisplayMode, OrderBookId, OrderBookInstance, OrderBookSymbolMode,
};
use crate::message::Message;

use iced::widget::{button, column, container, row, text, tooltip};
use iced::{Element, Fill, Theme};
use std::borrow::Cow;

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn view_order_book_header(
        id: OrderBookId,
        reverse_side: bool,
        total_in_notional: bool,
    ) -> Element<'static, Message> {
        let total = Self::view_order_book_total_header(id, "Total", total_in_notional);
        let headers = if reverse_side {
            row![total, header_cell("Size"), header_cell("Price")]
        } else {
            row![header_cell("Price"), header_cell("Size"), total]
        }
        .spacing(4)
        .align_y(iced::Alignment::Center);

        // Mirror the data rows' insets (4px row padding, plus the 15px
        // scrollbar gutter on the right) so the labels sit exactly over the
        // columns they name.
        container(headers)
            .height(24)
            .align_y(iced::alignment::Vertical::Center)
            .padding(iced::Padding {
                top: 0.0,
                right: 19.0,
                bottom: 0.0,
                left: 4.0,
            })
            .into()
    }

    pub(in crate::market_views::order_book) fn view_order_book_total_header(
        id: OrderBookId,
        label: &'static str,
        total_in_notional: bool,
    ) -> Element<'static, Message> {
        let label = if total_in_notional {
            format!("{label} ($)")
        } else {
            label.to_string()
        };
        tooltip(
            button(
                text(label)
                    .size(11)
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Right),
            )
            .width(Fill)
            .padding(0)
            .style(|theme: &Theme, status| {
                let colors = UiColors::new(theme);
                button::Style {
                    background: (status == button::Status::Hovered)
                        .then_some(mix_color(colors.panel, colors.text, 0.05).into()),
                    text_color: colors.muted,
                    ..Default::default()
                }
            })
            .on_press(Message::ToggleOrderBookTotalUnit(id)),
            text(if total_in_notional {
                "Show totals in base units"
            } else {
                "Show totals in USD"
            })
            .size(11),
            tooltip::Position::Bottom,
        )
        .into()
    }

    pub(in crate::market_views::order_book) fn view_order_book_title(
        &self,
        id: OrderBookId,
        inst: &OrderBookInstance,
    ) -> Element<'_, Message> {
        let is_active = matches!(inst.mode, OrderBookSymbolMode::Active);

        let tracking_label: Cow<'_, str> = match &inst.mode {
            OrderBookSymbolMode::Active => Cow::Borrowed(&self.active_symbol_display),
            OrderBookSymbolMode::Fixed(symbol) => Cow::Owned(self.display_name_for_symbol(symbol)),
        };

        let book_has_rows = !inst.book.bids.is_empty() || !inst.book.asks.is_empty();

        let tracking_text = text(tracking_label)
            .size(12)
            .style(move |theme: &Theme| text::Style {
                color: Some(UiColors::new(theme).text),
            });

        // The symbol-mode indicator carries its meaning in a tooltip: a
        // filled dot tracks the active (flicking) symbol; a pin marks a
        // symbol fixed to this book. The reserved 12px target keeps the
        // title layout identical in both modes.
        let symbol_mode_indicator: Element<'static, Message> = if is_active {
            tooltip(
                indicator_slot(icons::icon(icons::ACTIVE_DOT, 8.0, IconColor::Accent)),
                text("Tracks the active (selected) symbol").size(11),
                tooltip::Position::Bottom,
            )
            .into()
        } else {
            tooltip(
                indicator_slot(icons::icon(icons::PIN, 12.0, IconColor::Muted)),
                text("Pinned to this symbol").size(11),
                tooltip::Position::Bottom,
            )
            .into()
        };

        let title = row![symbol_mode_indicator, tracking_text]
            .spacing(2)
            .align_y(iced::Alignment::Center);

        // Fixed-size status badges live in the title row so background
        // refreshes and transient errors never reflow the book below.
        let mut status = row![].spacing(2).align_y(iced::Alignment::Center);
        if inst.book_loading && book_has_rows {
            status = status.push(self.view_spinner(12));
        }
        if let Some(error) = &inst.book_error
            && book_has_rows
        {
            status = status.push(stale_book_badge(error, self.pane_corner_radius));
        }

        let pane_corner_radius = self.pane_corner_radius;
        // Book/DOM/Depth is one segmented choice on a shared inset track, so
        // the three modes never change size, family, or density.
        let mode_track = container(
            row![
                display_mode_button(
                    id,
                    inst.display_mode,
                    OrderBookDisplayMode::DepthList,
                    "Book",
                    pane_corner_radius,
                ),
                display_mode_button(
                    id,
                    inst.display_mode,
                    OrderBookDisplayMode::DomLadder,
                    "DOM",
                    pane_corner_radius,
                ),
                display_mode_button(
                    id,
                    inst.display_mode,
                    OrderBookDisplayMode::DepthChart,
                    "Depth",
                    pane_corner_radius,
                ),
            ]
            .spacing(2),
        )
        .padding(2)
        .style(move |theme: &Theme| segment_track_style(theme, pane_corner_radius));

        let settings_button = button(icons::icon(icons::SETTINGS, 14.0, IconColor::Muted))
            .on_press(Message::ToggleOrderBookSettings(id))
            .width(24)
            .height(24)
            .padding(0)
            .style(move |theme: &Theme, status| {
                compact_button_style(theme, status, false, pane_corner_radius)
            });

        title
            .push(
                container(status)
                    .width(56)
                    .height(18)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .push(iced::widget::Space::new().width(Fill))
            .push(mode_track)
            .push(tooltip(
                settings_button,
                text("Book settings").size(11),
                tooltip::Position::Bottom,
            ))
            .align_y(iced::Alignment::Center)
            .into()
    }

    pub(in crate::market_views::order_book) fn view_order_book_outcome_metadata(
        &self,
        symbol: &str,
        inst: &OrderBookInstance,
    ) -> Option<Element<'static, Message>> {
        let exchange_symbol = self.exchange_symbols.iter().find(|sym| sym.key == symbol)?;
        let info = exchange_symbol.outcome.as_ref()?;
        let condition = info.side_condition_label_with_countdown(self.status_bar_now_ms);
        let probability = inst
            .current_mid_price()
            .or_else(|| self.resolve_mid_for_symbol_at(symbol, self.status_bar_now_ms))
            .filter(|mid| mid.is_finite() && (0.0..=1.0).contains(mid))
            .map(|mid| {
                if info.contract.scalar {
                    format!("{mid:.3} {}", info.quote_symbol)
                } else {
                    format!("{:.1}% implied", mid * 100.0)
                }
            })
            .unwrap_or_else(|| "mid n/a".to_string());
        let token_name = format!("+{}", info.encoding);
        let detail = format!(
            "{} | token {} | asset {} | quote {} | whole contracts",
            symbol, token_name, exchange_symbol.asset_index, info.quote_symbol
        );
        let mut metadata = column![
            row![
                text(format!("{}: {condition}", info.side_name))
                    .size(11)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).text),
                    })
                    .width(Fill),
                text(probability)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).text),
                    }),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
            text(detail)
                .size(11)
                .style(move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).muted),
                }),
        ]
        .spacing(2);
        if let Some(reason) = info.trading_block_reason(self.status_bar_now_ms) {
            metadata = metadata.push(text(reason.to_string()).size(11).style(
                move |theme: &Theme| text::Style {
                    color: Some(UiColors::new(theme).negative),
                },
            ));
        }
        if let Some(source) = info.settlement_source_label() {
            metadata =
                metadata.push(
                    text(source)
                        .size(11)
                        .style(move |theme: &Theme| text::Style {
                            color: Some(UiColors::new(theme).muted),
                        }),
                );
        }

        let pane_corner_radius = self.pane_corner_radius;
        Some(
            container(metadata)
                .width(Fill)
                .padding([4, 6])
                .style(move |theme: &Theme| container::Style {
                    background: Some(UiColors::new(theme).inset.into()),
                    border: iced::Border {
                        radius: effective_radius(pane_corner_radius, 6.0).into(),
                        width: 1.0,
                        color: UiColors::new(theme).divider,
                    },
                    ..Default::default()
                })
                .into(),
        )
    }

    pub(in crate::market_views::order_book) fn view_order_book_spread_chart<'a>(
        id: OrderBookId,
        inst: &'a OrderBookInstance,
    ) -> Element<'a, Message> {
        let mid = inst.book.mid_price();
        let spread_decimals = helpers::tick_decimals(helpers::default_tick_for_price(mid));

        iced::widget::canvas(crate::spread_chart::SpreadChart {
            id,
            data: &inst.spread_history,
            spread_decimals,
        })
        .width(Fill)
        .height(iced::Length::Fixed(inst.spread_chart_height))
        .into()
    }
}

fn header_cell(label: &'static str) -> Element<'static, Message> {
    text(label)
        .size(11)
        .width(Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .style(move |theme: &Theme| text::Style {
            color: Some(UiColors::new(theme).muted),
        })
        .into()
}

/// Reserved, centered 12px slot for the symbol-mode indicator so the active
/// dot and the pinned pin keep the title layout stable.
fn indicator_slot(icon: Element<'static, Message>) -> Element<'static, Message> {
    container(icon)
        .width(12)
        .align_x(iced::alignment::Horizontal::Center)
        .into()
}

/// Inset track behind a segmented choice. Segments render their own
/// selected, hover, and pressed surfaces on top of it.
fn segment_track_style(theme: &Theme, pane_corner_radius: f32) -> container::Style {
    container::Style {
        background: Some(UiColors::new(theme).inset.into()),
        border: iced::Border {
            radius: effective_radius(pane_corner_radius, 4.0).into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Compact constant-size indicator that the displayed book is a stale
/// snapshot; the full error message lives in the tooltip.
fn stale_book_badge(error: &str, pane_corner_radius: f32) -> Element<'static, Message> {
    const SOFT_WARNING_FILL: f32 = 0.18;
    let badge = container(text("Stale").size(11).style(move |theme: &Theme| {
        let colors = UiColors::new(theme);
        let fill = mix_color(colors.panel, colors.warning, SOFT_WARNING_FILL);
        text::Style {
            color: Some(readable_color(colors.warning, fill, 4.5)),
        }
    }))
    .padding([1, 6])
    .height(18)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |theme: &Theme| {
        let colors = UiColors::new(theme);
        container::Style {
            background: Some(mix_color(colors.panel, colors.warning, SOFT_WARNING_FILL).into()),
            border: iced::Border {
                radius: effective_radius(pane_corner_radius, 3.0).into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    tooltip(
        badge,
        container(text(format!("{error}; showing last snapshot")).size(11))
            .padding([6, 8])
            .max_width(280)
            .style(move |theme: &Theme| UiColors::new(theme).overlay(pane_corner_radius)),
        tooltip::Position::Bottom,
    )
    .into()
}

fn display_mode_button(
    id: OrderBookId,
    active: OrderBookDisplayMode,
    mode: OrderBookDisplayMode,
    label: &'static str,
    pane_corner_radius: f32,
) -> button::Button<'static, Message> {
    let is_active = active == mode;
    button(text(label).size(12).center())
        .on_press(Message::SetOrderBookDisplayMode(id, mode))
        .height(24)
        .padding([0, 6])
        .style(move |theme: &Theme, status| {
            compact_button_style(theme, status, is_active, pane_corner_radius)
        })
}
