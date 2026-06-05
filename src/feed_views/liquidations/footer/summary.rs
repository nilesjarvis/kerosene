use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, column, container, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_liquidations_summary(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let timeframes = [(1, "1m"), (5, "5m"), (15, "15m"), (60, "1H")];
        let mut summary_row = row![].spacing(16);
        for (mins, label) in timeframes.iter() {
            let (l_notional, s_notional) = self.calculate_liquidation_summary(*mins, now_ms);
            let total = l_notional + s_notional;
            let has_data = total > 0.0;

            let (text_l_color, text_s_color, bar_l_color, bar_s_color) = if has_data {
                (
                    theme.palette().danger,
                    theme.palette().success,
                    theme.palette().danger,
                    theme.palette().success,
                )
            } else {
                let gray_text = theme.extended_palette().background.weak.text;
                let gray_bar = theme.extended_palette().background.strong.color;
                (gray_text, gray_text, gray_bar, gray_bar)
            };

            let total_str = denomination.format_value(total, 0);
            let total_color = if has_data {
                theme.palette().text
            } else {
                theme.extended_palette().background.weak.text
            };

            let text_block = column![
                row![
                    text(*label)
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                    Space::new().width(Fill),
                    text(total_str).size(11).color(total_color),
                ]
                .width(Fill),
                row![
                    text(format!("L: {}", denomination.format_value(l_notional, 0)))
                        .size(10)
                        .color(text_l_color),
                    Space::new().width(Fill),
                    text(format!("S: {}", denomination.format_value(s_notional, 0)))
                        .size(10)
                        .color(text_s_color),
                ]
                .width(Fill),
            ]
            .spacing(2)
            .width(Fill);

            let (l_ratio, s_ratio) = if has_data {
                ((l_notional / total) as f32, (s_notional / total) as f32)
            } else {
                (0.5, 0.5)
            };

            let bar_height = 4.0;

            let ratio_bar = row![
                container(Space::new())
                    .width(iced::Length::FillPortion((l_ratio * 1000.0).max(1.0) as u16))
                    .height(bar_height)
                    .style(move |_| container_style::Style {
                        background: Some(bar_l_color.into()),
                        ..Default::default()
                    }),
                container(Space::new())
                    .width(iced::Length::FillPortion((s_ratio * 1000.0).max(1.0) as u16))
                    .height(bar_height)
                    .style(move |_| container_style::Style {
                        background: Some(bar_s_color.into()),
                        ..Default::default()
                    }),
            ]
            .width(Fill)
            .height(bar_height);

            let block = column![text_block, ratio_bar].spacing(4).width(Fill);
            summary_row = summary_row.push(block);
        }

        container(summary_row)
            .width(Fill)
            .padding(8)
            .style(move |theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}
