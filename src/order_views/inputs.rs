use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::OrderKind;

use iced::widget::{Column, Space, column, row, text};
use iced::{Element, Fill, Theme};

mod fees;
mod price;
mod size;
mod warnings;

// ---------------------------------------------------------------------------
// Order Inputs
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_order_input_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
        wide: bool,
    ) -> Column<'a, Message> {
        let inputs = self.push_price_input_controls(column![].spacing(8));
        let (inputs, notional_val) = self.push_size_input_controls(inputs, active_is_outcome, wide);
        let theme = self.theme();
        let mut details = column![order_detail_row(
            "Order value",
            notional_val
                .map(|value| crate::helpers::format_usd(&format!("{value:.2}")))
                .unwrap_or_else(|| "\u{2014}".to_string()),
            &theme,
        )]
        .spacing(6);
        if self.order_kind != OrderKind::Twap {
            details = self.push_fee_estimate(details, active_is_spot, active_is_outcome);
        }
        details = self.push_order_execution_options(details, active_is_spot, active_is_outcome);
        details =
            self.push_leverage_warning(details, active_is_spot, active_is_outcome, notional_val);
        details = self.push_order_algorithm_settings(details);
        details = self.push_order_presets_menu(details, active_is_outcome);

        if wide {
            form.push(row![inputs.width(Fill), details.width(Fill)].spacing(16))
        } else {
            form.push(inputs).push(details)
        }
    }
}

fn order_detail_row(
    label: &'static str,
    value: String,
    theme: &Theme,
) -> Element<'static, Message> {
    row![
        text(label)
            .size(11)
            .color(theme.extended_palette().background.weak.text),
        Space::new().width(Fill),
        text(value)
            .size(11)
            .font(crate::app_fonts::monospace_font()),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}
