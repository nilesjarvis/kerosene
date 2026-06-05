mod actions;
mod connection;
mod toggles;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, float, row, stack};
use iced::{Element, Fill, Vector};

impl TradingTerminal {
    pub(crate) fn view_liquidations_top_bar(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();

        let controls = row![
            self.view_liquidations_connection_controls(now_ms, &theme),
            Space::new().width(Fill),
            self.view_liquidation_threshold_controls(&theme),
            self.view_liquidation_follow_button(),
            self.view_liquidation_settings_button(),
            Space::new().width(8),
            self.view_clear_liquidations_button(),
        ]
        .spacing(8)
        .width(Fill)
        .align_y(iced::Alignment::Center);

        let mut layers: Vec<Element<'_, Message>> = vec![controls.into()];
        if self.liquidation_settings_menu_open {
            let dropdown_layer = float(
                row![
                    Space::new().width(Fill),
                    self.view_liquidation_settings_dropdown(),
                ]
                .width(Fill)
                .align_y(iced::Alignment::Center),
            )
            .translate(|bounds, _viewport| Vector::new(0.0, bounds.height + 6.0));

            layers.push(dropdown_layer.into());
        }

        stack(layers).width(Fill).into()
    }
}
