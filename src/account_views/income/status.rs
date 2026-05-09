use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, button, column, container, row, rule, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_income_title(&self) -> Element<'_, Message> {
        let income_alerts_enabled = self.income_alerts_enabled;
        let alerts_btn = button(
            text(if income_alerts_enabled {
                "Interest Alerts: ON"
            } else {
                "Interest Alerts: OFF"
            })
            .size(10),
        )
        .on_press(Message::ToggleIncomeAlerts)
        .padding([2, 8])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if income_alerts_enabled {
                    theme.palette().success
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        row![
            text("Income").size(13),
            crate::helpers::vertical_spacer(),
            button(text("Refresh").size(10))
                .on_press(Message::RefreshIncome)
                .padding([2, 8]),
            alerts_btn,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    pub(super) fn view_income_unavailable(&self) -> Column<'_, Message> {
        let theme = self.theme();
        column![
            self.view_income_title(),
            rule::horizontal(1),
            container(
                text("Income widget is available in Portfolio Margin mode only")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(180)
            .center(Fill),
        ]
        .spacing(8)
    }

    pub(super) fn view_income_loading(&self) -> Column<'_, Message> {
        column![
            self.view_income_title(),
            self.loading_overlay("Loading income...")
        ]
        .spacing(8)
    }

    pub(super) fn view_income_empty(&self) -> Column<'_, Message> {
        let theme = self.theme();
        let mut content = column![
            self.view_income_title(),
            container(
                text("No income data available")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
            )
            .width(Fill)
            .height(200)
            .center(Fill),
        ]
        .spacing(8);

        if let Some(err) = &self.income.last_error {
            content = content.push(
                text(format!("Stale data: {err}"))
                    .size(11)
                    .color(theme.palette().primary),
            );
        }

        content
    }
}
