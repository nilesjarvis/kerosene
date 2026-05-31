mod grid;
mod panes;
mod title_bar;
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
        self.view_main_with_top_bar(self.view_account_summary_bar())
    }

    pub(crate) fn view_main_with_top_bar<'a>(
        &'a self,
        top_bar: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let theme = self.theme();

        let mut main_column = column![top_bar]
            .spacing(self.pane_border_thickness)
            .width(Fill)
            .height(Fill);
        if self.ticker_tape_enabled {
            main_column = main_column.push(self.view_ticker_tape_bar());
        }
        let main_content: Element<'_, Message> =
            main_column.push(self.view_main_pane_grid()).into();

        let mut layers: Vec<Element<'_, Message>> = vec![main_content];

        if let Some(toast_overlay) = self.view_toast_overlay(&theme) {
            layers.push(toast_overlay);
        }

        if let Some(alfred_overlay) = self.view_alfred_overlay(&theme) {
            layers.push(alfred_overlay);
        }

        if self.show_unlock_credentials_popup && self.encrypted_credentials_locked() {
            layers.push(self.view_unlock_credentials_popup());
        }

        let main_stack: Element<'_, Message> = stack(layers).width(Fill).height(Fill).into();

        container(
            column![main_stack, self.view_status_bar()]
                .spacing(self.pane_border_thickness)
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
