mod grid;
mod panes;
mod unlock;
mod windows;

use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{column, container, stack};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Main window shell
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_main(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let can_add_income = self
            .account_data
            .as_ref()
            .is_some_and(AccountData::is_portfolio_margin);

        let mut layers: Vec<Element<'_, Message>> = vec![self.view_main_pane_grid()];

        if self.account_picker_open {
            let account_menu: Element<'_, Message> = container(self.view_account_picker_dropdown())
                .width(Fill)
                .padding(iced::Padding {
                    top: 42.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 16.0,
                })
                .align_x(iced::Alignment::Start)
                .into();

            layers.push(account_menu);
        }

        if let Some(menu) = self.view_add_widget_menu(&theme, can_add_income) {
            layers.push(menu);
        }

        if let Some(toast_overlay) = self.view_toast_overlay(&theme) {
            layers.push(toast_overlay);
        }

        if self.show_unlock_credentials_popup && self.encrypted_credentials_locked() {
            layers.push(self.view_unlock_credentials_popup());
        }

        let main_stack: Element<'_, Message> = stack(layers).width(Fill).height(Fill).into();

        container(
            column![main_stack, self.view_status_bar()]
                .width(Fill)
                .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            text_color: Some(theme.palette().text),
            ..Default::default()
        })
        .into()
    }
}
