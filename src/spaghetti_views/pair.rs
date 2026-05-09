use crate::app_state::TradingTerminal;
use crate::helpers::{self, timeframe_button};
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};

use iced::widget::container as container_style;
use iced::widget::{column, container, row, text, text_input};
use iced::{Color, Element, Theme};

mod execute;

impl TradingTerminal {
    pub(super) fn view_spaghetti_pair_controls<'a>(
        &'a self,
        id: SpaghettiChartId,
        inst: &'a SpaghettiChartInstance,
        theme: &Theme,
    ) -> Option<Element<'a, Message>> {
        if !inst.pair_mode {
            return None;
        }

        let has_two = inst.canvas.series.len() >= 2;
        let pair_label = if has_two {
            format!(
                "{} / {}",
                inst.canvas.series[0].display, inst.canvas.series[1].display
            )
        } else {
            "Add two symbols to enable pair trade".to_string()
        };

        let notional_input = text_input("Notional per leg (USD)", &inst.pair_notional)
            .style(helpers::text_input_style)
            .on_input(move |v| Message::PairNotionalChanged(id, v))
            .size(11)
            .padding([4, 6]);

        let mode_line = timeframe_button(
            "Line",
            !inst.pair_candle_mode,
            Message::PairSetCandleMode(id, false),
        );
        let mode_candle = timeframe_button(
            "Candles",
            inst.pair_candle_mode,
            Message::PairSetCandleMode(id, true),
        );
        let reset_view = timeframe_button("Reset View", false, Message::SpaghettiResetView(id));
        let can_trade = self.can_execute_spaghetti_pair_trade(inst);
        let long_short_btn = self.view_pair_execute_button(
            "Long A / Short B",
            Message::PairExecute(id, true),
            can_trade,
            inst.pair_pending,
            true,
        );
        let short_long_btn = self.view_pair_execute_button(
            "Short A / Long B",
            Message::PairExecute(id, false),
            can_trade,
            inst.pair_pending,
            false,
        );

        let panel_text_color = theme.palette().text;
        let panel_background = theme.extended_palette().background.strong.color;
        let panel_border_color = theme.palette().primary;

        let panel = container(
            column![
                text(pair_label).size(11).color(panel_text_color),
                row![mode_line, mode_candle]
                    .spacing(4)
                    .align_y(iced::Alignment::Center),
                row![reset_view].spacing(4).align_y(iced::Alignment::Center),
                row![notional_input].spacing(8),
                row![long_short_btn, short_long_btn]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
            ]
            .spacing(6),
        )
        .padding([6, 8])
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(
                Color {
                    a: 0.65,
                    ..panel_background
                }
                .into(),
            ),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.3,
                    ..panel_border_color
                },
            },
            ..Default::default()
        });
        Some(panel.into())
    }
}
