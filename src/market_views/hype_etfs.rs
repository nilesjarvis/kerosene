use crate::app_state::TradingTerminal;
use crate::hype_etf_state::HypeEtfView;
use crate::message::Message;

use iced::widget::{button, column, container, responsive, row, rule, text};
use iced::{Color, Element, Fill, Theme};

mod body;
mod chart;
mod formatting;
mod metrics;
mod sections;

#[cfg(test)]
use chart::*;

// ---------------------------------------------------------------------------
// HYPE ETF View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_hype_etfs(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_hype_etfs_sized(size.width)).into()
    }

    fn view_hype_etfs_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let header = row![
            text("HYPE ETFs")
                .size(13)
                .color(theme.palette().text)
                .width(Fill),
            button(text("Refresh").size(11).center())
                .padding([3, 8])
                .on_press(Message::RefreshHypeEtfs)
                .style(button::text),
        ]
        .align_y(iced::Alignment::Center);

        let mut content = column![header, self.view_hype_etf_tabs(), rule::horizontal(1)]
            .spacing(8)
            .width(Fill);

        content = content.push(self.view_hype_etfs_body(available_width));

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn view_hype_etf_tabs(&self) -> Element<'static, Message> {
        HypeEtfView::ALL
            .iter()
            .copied()
            .fold(row![].spacing(4), |tabs, view| {
                tabs.push(hype_etf_tab(view, self.hype_etfs.view == view))
            })
            .into()
    }
}

fn hype_etf_tab(view: HypeEtfView, active: bool) -> Element<'static, Message> {
    button(text(view.label()).size(11).center().width(Fill))
        .on_press(Message::HypeEtfsViewChanged(view))
        .padding([4, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = match (active, status) {
                (true, _) => theme.extended_palette().background.strong.color,
                (false, button::Status::Hovered) => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    width: if active { 1.0 } else { 0.0 },
                    color: if active {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
}

#[cfg(test)]
mod tests;
