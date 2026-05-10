mod grid;
mod panes;
mod unlock;
mod windows;

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

        let mut layers: Vec<Element<'_, Message>> = vec![self.view_main_pane_grid()];

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
