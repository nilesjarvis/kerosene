use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, column, container, row, text, tooltip};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_liquidations_chart(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let current_sec = now_ms / 1000;
        let mut chart_bars = row![].spacing(2).width(Fill);
        let num_bars = 60;
        let bar_width = iced::Length::Fill;

        let mut max_vol = 0.0;
        let mut chart_data = Vec::with_capacity(num_bars);
        for i in (0..num_bars).rev() {
            let sec = current_sec.saturating_sub(i as u64);
            let (l, s) = self
                .liquidation_chart_buckets
                .get(&sec)
                .copied()
                .unwrap_or((0.0, 0.0));
            if l > max_vol {
                max_vol = l;
            }
            if s > max_vol {
                max_vol = s;
            }
            chart_data.push((l, s));
        }
        if max_vol == 0.0 {
            max_vol = 1.0;
        }

        let max_bar_height = 24.0;
        let success_color = theme.palette().success;
        let danger_color = theme.palette().danger;

        for (l, s) in chart_data {
            let s_h = ((s / max_vol) as f32 * max_bar_height).max(0.0);
            let l_h = ((l / max_vol) as f32 * max_bar_height).max(0.0);

            let s_fill = if s > 0.0 { s_h.max(2.0) } else { 0.0 };
            let l_fill = if l > 0.0 { l_h.max(2.0) } else { 0.0 };

            let bar = column![
                container(Space::new()).height(iced::Length::Fixed(max_bar_height - s_fill)),
                container(Space::new())
                    .width(bar_width)
                    .height(iced::Length::Fixed(s_fill))
                    .style(move |_| container_style::Style {
                        background: Some(success_color.into()),
                        ..Default::default()
                    }),
                container(Space::new())
                    .width(bar_width)
                    .height(iced::Length::Fixed(l_fill))
                    .style(move |_| container_style::Style {
                        background: Some(danger_color.into()),
                        ..Default::default()
                    }),
            ]
            .width(bar_width)
            .height(iced::Length::Fixed(max_bar_height * 2.0));

            let tooltip_text = if l > 0.0 || s > 0.0 {
                format!(
                    "L: {}\nS: {}",
                    denomination.format_value(l, 0),
                    denomination.format_value(s, 0)
                )
            } else {
                "No data".to_string()
            };

            let wrapped_bar = tooltip(
                bar,
                text(tooltip_text)
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
                iced::widget::tooltip::Position::Top,
            )
            .gap(4)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                text_color: Some(theme.palette().text),
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

            chart_bars = chart_bars.push(wrapped_bar);
        }

        container(chart_bars)
            .width(Fill)
            .padding([4, 8])
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
