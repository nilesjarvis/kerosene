mod cells;
mod display_values;
mod formatting;
#[cfg(test)]
mod tests;

use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, hip3_dex};
use crate::message::Message;

use super::super::{
    POSITION_ACTION_WIDTH, POSITION_ENTRY_WIDTH, POSITION_FUNDING_WIDTH, POSITION_LEVERAGE_WIDTH,
    POSITION_LIQ_WIDTH, POSITION_MARK_WIDTH, POSITION_SIDE_WIDTH, PositionColumnVisibility,
    PositionNumberMode,
};
use super::format_position_compact_number;
use super::sort::PositionRowData;
use cells::{position_symbol_button, position_upnl_cell};
#[cfg(test)]
use formatting::format_position_signed_amount;
use formatting::{
    format_position_entry_price, format_spot_position_entry_price, trim_decimal_zeros,
};
use iced::widget::text::Wrapping;
use iced::widget::{container, row, text};
use iced::{Element, Fill, Theme, color};

impl TradingTerminal {
    pub(super) fn view_position_row<'a>(
        &'a self,
        data: PositionRowData,
        can_close: bool,
        theme: &Theme,
        columns: PositionColumnVisibility,
        number_mode: PositionNumberMode,
    ) -> Element<'a, Message> {
        let ap = &data.ap;
        let pos = &ap.position;
        let denomination = self.display_denomination_context();
        let is_spot_position = self.is_spot_coin(&pos.coin);
        let weak_text = theme.extended_palette().background.weak.text;
        let (side, side_color) = match data.is_long {
            _ if is_spot_position => ("Spot", weak_text),
            Some(true) => ("\u{2191} Long", theme.palette().success),
            Some(false) => ("\u{2193} Short", theme.palette().danger),
            None => ("Invalid", theme.palette().warning),
        };

        let mark_str = data
            .mark_px
            .map(format_price)
            .unwrap_or_else(|| "\u{2014}".to_string());
        let entry_str =
            if is_spot_position && data.entry_px.is_none() && pos.entry_px.trim().is_empty() {
                "-".to_string()
            } else if is_spot_position {
                format_spot_position_entry_price(data.entry_px)
            } else {
                format_position_entry_price(data.entry_px, &pos.entry_px)
            };
        let size_str = data
            .szi
            .map(|szi| self.display_position_size(&pos.coin, szi.abs(), number_mode))
            .unwrap_or_else(|| "Invalid".to_string());

        let pnl_color = data
            .upnl
            .map(|upnl| self.direction_color(theme, upnl))
            .unwrap_or_else(|| {
                if is_spot_position {
                    weak_text
                } else {
                    theme.palette().warning
                }
            });
        let lev_str = format!("{}x {}", pos.leverage.value, pos.leverage.leverage_type);
        let liq_element: Element<'a, Message> = text(
            data.liq_px
                .map(format_price)
                .unwrap_or_else(|| "\u{2014}".to_string()),
        )
        .size(12)
        .font(crate::app_fonts::monospace_font())
        .color(color!(0xff9d66))
        .wrapping(Wrapping::None)
        .into();

        let funding_color = match data.funding_since_open {
            Some(funding) if funding > 0.0 => self.direction_color(theme, funding),
            Some(funding) if funding < 0.0 => self.direction_color(theme, funding),
            Some(_) => theme.extended_palette().background.weak.text,
            None => theme.extended_palette().background.weak.text,
        };
        let total_pnl_color = data
            .total_pnl
            .map(|total_pnl| self.direction_color(theme, total_pnl))
            .unwrap_or_else(|| {
                if is_spot_position {
                    weak_text
                } else {
                    theme.palette().warning
                }
            });

        let row_can_close = can_close && data.szi.is_some_and(|szi| szi.abs() > 1e-12);
        let is_hidden = self.position_is_hidden(&pos.coin);
        let close_cell =
            self.view_position_close_cell(pos.coin.clone(), row_can_close, is_hidden, theme);
        let pnl_displays = self.position_row_pnl_displays(&data, &denomination, number_mode);
        let symbol_icon_key = self.position_row_symbol_icon_key(&pos.coin);
        let symbol_btn = position_symbol_button(
            &pos.coin,
            symbol_icon_key,
            self.position_row_symbol_label(&pos.coin),
            self.position_row_symbol_exchange_label(&pos.coin),
            theme,
        );
        let upnl_cell = position_upnl_cell(&pos.coin, pnl_displays.upnl, pnl_color);

        // Optimistic account updates: annotate the size with the projected
        // value while market orders for this symbol are in flight. A position
        // whose own size failed to parse has no trustworthy baseline, so it
        // gets no projection.
        let projected_label = data.szi.and_then(|szi| {
            self.optimistic_position_delta_for_symbol(&pos.coin)
                .map(|delta| {
                    format!(
                        "{size_str} \u{2192} {}",
                        self.projected_position_size_label(&pos.coin, szi, delta, number_mode)
                    )
                })
        });
        let size_cell: Element<'a, Message> = match projected_label {
            Some(label) => text(label)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().primary)
                .width(Fill)
                .wrapping(Wrapping::None)
                .into(),
            None => text(size_str)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .width(Fill)
                .wrapping(Wrapping::None)
                .into(),
        };

        let mut row_content = row![
            container(symbol_btn).width(Fill),
            text(side)
                .size(12)
                .color(side_color)
                .width(POSITION_SIDE_WIDTH)
                .wrapping(Wrapping::None),
            size_cell,
        ];
        if columns.entry {
            row_content = row_content.push(
                text(entry_str)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .width(POSITION_ENTRY_WIDTH)
                    .wrapping(Wrapping::None),
            );
        }
        if columns.liquidation {
            row_content = row_content.push(container(liq_element).width(POSITION_LIQ_WIDTH));
        }
        if columns.mark {
            row_content = row_content.push(
                text(mark_str)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .width(POSITION_MARK_WIDTH)
                    .wrapping(Wrapping::None),
            );
        }
        row_content = row_content
            .push(
                text(pnl_displays.value)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .width(Fill)
                    .wrapping(Wrapping::None),
            )
            .push(container(upnl_cell).width(Fill));
        if columns.funding {
            row_content = row_content.push(
                text(pnl_displays.funding)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .color(funding_color)
                    .width(POSITION_FUNDING_WIDTH)
                    .wrapping(Wrapping::None),
            );
        }
        if columns.total_pnl {
            row_content = row_content.push(
                text(pnl_displays.total)
                    .size(13)
                    .font(crate::app_fonts::monospace_font())
                    .color(total_pnl_color)
                    .width(Fill)
                    .wrapping(Wrapping::None),
            );
        }
        if columns.leverage {
            row_content = row_content.push(
                text(lev_str)
                    .size(12)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text)
                    .width(POSITION_LEVERAGE_WIDTH)
                    .wrapping(Wrapping::None),
            );
        }
        row_content = row_content
            .push(container(close_cell).width(POSITION_ACTION_WIDTH))
            .spacing(4)
            .align_y(iced::Alignment::Center);

        container(row_content)
            .width(Fill)
            .padding([6, 8])
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

    /// Projected size for an in-flight market-order delta, with the direction
    /// made explicit whenever it differs from the current side: an oversized
    /// opposite-side order flips the position, and rendering only the
    /// magnitude would show "1 \u{2192} 1" for a long reversed into a short.
    fn projected_position_size_label(
        &self,
        coin: &str,
        szi: f64,
        delta: f64,
        number_mode: PositionNumberMode,
    ) -> String {
        const FLAT_EPSILON: f64 = 1e-12;
        let projected = szi + delta;
        let size_label = self.display_position_size(coin, projected.abs(), number_mode);
        if projected.abs() <= FLAT_EPSILON {
            return "0".to_string();
        }
        if szi.abs() > FLAT_EPSILON && projected.signum() != szi.signum() {
            let side = if projected > 0.0 { "Long" } else { "Short" };
            return format!("{size_label} ({side})");
        }
        size_label
    }

    fn display_position_size(
        &self,
        coin: &str,
        size: f64,
        number_mode: PositionNumberMode,
    ) -> String {
        if !number_mode.is_compact() {
            return self.display_size_for_symbol(coin, size);
        }

        if size >= 10_000.0 {
            format_position_compact_number(size)
        } else {
            trim_decimal_zeros(self.display_size_for_symbol(coin, size))
        }
    }

    fn position_row_symbol_label(&self, coin: &str) -> String {
        if self.is_outcome_coin(coin) {
            if let Some(symbol) = self
                .exchange_symbols
                .iter()
                .find(|symbol| symbol.key == coin)
            {
                return Self::exchange_symbol_display_name(symbol);
            }
            return self.display_name_for_symbol(coin);
        }
        if self.is_spot_coin(coin) {
            return self
                .exchange_symbol_for_key(coin)
                .map(|symbol| symbol.ticker.clone())
                .unwrap_or_else(|| self.display_name_for_symbol(coin));
        }
        if hip3_dex(coin).is_some() {
            return self.display_name_for_symbol(coin);
        }

        coin.to_string()
    }

    fn position_row_symbol_icon_key<'a>(&'a self, coin: &'a str) -> &'a str {
        if self.is_spot_coin(coin) {
            return self
                .exchange_symbol_for_key(coin)
                .map(|symbol| symbol.ticker.as_str())
                .unwrap_or(coin);
        }

        coin
    }

    fn position_row_symbol_exchange_label(&self, coin: &str) -> Option<String> {
        if self.is_outcome_coin(coin) || self.is_spot_coin(coin) {
            return None;
        }

        hip3_dex(coin).map(str::to_string)
    }
}
