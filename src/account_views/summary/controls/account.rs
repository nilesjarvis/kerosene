use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Element;
use iced::widget::text;

// ---------------------------------------------------------------------------
// Account Summary Profile Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn summary_account_picker(&self) -> Element<'_, Message> {
        let theme = self.theme();
        self.view_account_picker_button(&theme)
    }

    pub(crate) fn summary_secret_status(&self) -> Option<Element<'_, Message>> {
        let theme = self.theme();
        self.secret_store_status.as_ref().map(|(status, is_error)| {
            text(status)
                .size(10)
                .color(if *is_error {
                    theme.palette().danger
                } else {
                    theme.extended_palette().background.weak.text
                })
                .into()
        })
    }
}
