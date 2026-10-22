use crate::app_state::TradingTerminal;
use crate::helpers::positive_finite_value;
use crate::message::Message;
use iced::widget::{Column, container, row, text};
use iced::{Color, Fill, Theme};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Input Warnings
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_leverage_warning<'a>(
        &'a self,
        form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
        notional_val: Option<f64>,
    ) -> Column<'a, Message> {
        let Some((warning_text, warning_color)) =
            self.leverage_warning(active_is_spot, active_is_outcome, notional_val)
        else {
            return form;
        };

        let pane_corner_radius = self.pane_corner_radius;
        // Inline warning notice: shared soft-warning fill with a readable
        // warning foreground and a vector state marker.
        let warning = container(
            row![
                crate::helpers::icons::icon(
                    crate::helpers::icons::WARNING,
                    14.0,
                    crate::helpers::icons::IconColor::Warning,
                ),
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
        .style(move |theme: &Theme| {
            let colors = crate::helpers::design::UiColors::new(theme);
            container::Style {
                background: Some(
                    crate::helpers::design::mix_color(colors.panel, colors.warning, 0.14).into(),
                ),
                border: iced::Border {
                    radius: crate::config::effective_radius(pane_corner_radius, 4.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        form.push(warning)
    }

    /// Margin-based leverage warning for the order ticket. Spot and outcome
    /// markets have no leverage, so notional there is unrelated to USDC
    /// margin and never warrants this warning.
    fn leverage_warning(
        &self,
        active_is_spot: bool,
        active_is_outcome: bool,
        notional_val: Option<f64>,
    ) -> Option<(String, Color)> {
        let theme = self.theme();
        if active_is_spot || active_is_outcome {
            return None;
        }

        let notional_val = notional_val.and_then(positive_finite_value)?;
        let (_, data) = self.connected_order_account_snapshot()?;

        match self.visible_available_margin_usdc(data) {
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
        }
    }
}
