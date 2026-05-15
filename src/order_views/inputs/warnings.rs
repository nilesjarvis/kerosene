use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, container, row, text};
use iced::{Color, Fill, Theme};

// ---------------------------------------------------------------------------
// Input Warnings
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_leverage_warning<'a>(
        &'a self,
        form: Column<'a, Message>,
        active_is_outcome: bool,
        notional_val: Option<f64>,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        if active_is_outcome {
            return form;
        }

        let Some(notional_val) = notional_val.filter(|value| value.is_finite() && *value > 0.0)
        else {
            return form;
        };

        let Some(data) = &self.account_data else {
            return form;
        };

        let warning = match self.visible_available_margin_usdc(data) {
            None => Some((
                "Account margin data unavailable".to_string(),
                theme.palette().warning,
            )),
            Some(available_margin)
                if notional_val <= available_margin * 2.0 || available_margin <= 0.0 =>
            {
                None
            }
            Some(available_margin) => Some((
                format!(
                    "High Leverage: {:.1}x Notional",
                    notional_val / available_margin
                ),
                theme.palette().danger,
            )),
        };

        let Some((warning_text, warning_color)) = warning else {
            return form;
        };

        let warning = container(
            row![
                text("\u{26a0}\u{fe0f}").size(14),
                text(warning_text)
                    .size(11)
                    .color(warning_color)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::DEFAULT
                    })
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([6, 8])
        .style(move |_theme: &Theme| container::Style {
            background: Some(
                Color {
                    a: 0.1,
                    ..warning_color
                }
                .into(),
            ),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.5,
                    ..warning_color
                },
            },
            ..Default::default()
        });

        form.push(warning)
    }
}
