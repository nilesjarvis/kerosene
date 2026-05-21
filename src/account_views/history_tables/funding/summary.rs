use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{row, text};
use iced::{Element, Theme, color};

impl TradingTerminal {
    pub(super) fn view_funding_total_label(
        &self,
        total_funding: Option<f64>,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let total_color = match total_funding {
            Some(total_funding) if total_funding >= 0.0 => theme.palette().success,
            Some(_) => theme.palette().danger,
            None => theme.palette().warning,
        };
        let denomination = self.display_denomination_context();
        let total_display = funding_total_display(&denomination, total_funding, self.hide_pnl);

        row![
            text("7d Total:")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            text(total_display)
                .font(crate::app_fonts::monospace_font())
                .size(11)
                .color(total_color),
            text(denomination.active_code().to_string())
                .size(10)
                .color(color!(0x666666)),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

fn funding_total_display(
    denomination: &crate::denomination::DisplayDenominationContext,
    total_funding: Option<f64>,
    hide_pnl: bool,
) -> String {
    if hide_pnl {
        "***".to_string()
    } else {
        total_funding
            .map(|total_funding| denomination.format_signed_value(total_funding, 4))
            .unwrap_or_else(|| "Invalid data".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funding_total_display_marks_invalid_values() {
        let denomination = crate::denomination::DisplayDenominationContext::default();
        assert_eq!(
            funding_total_display(&denomination, Some(1.25), false),
            "+$1.2500"
        );
        assert_eq!(
            funding_total_display(&denomination, Some(-1.25), false),
            "-$1.2500"
        );
        assert_eq!(
            funding_total_display(&denomination, None, false),
            "Invalid data"
        );
        assert_eq!(funding_total_display(&denomination, None, true), "***");
    }
}
