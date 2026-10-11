use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, checkbox, column, row, text, text_input};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(super) fn view_builder_code_settings(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let settings = &self.builder_codes;
        let saved = settings.draft_is_saved();
        let account = self.builder_approval_account();
        let available = settings.pending.is_none() && saved && account.is_ok();
        let fee_preview = crate::builder_codes::BuilderFee::from_inputs(
            &settings.address_input,
            &settings.fee_input,
        )
        .ok()
        .map(|fee| fee.max_fee_rate())
        .unwrap_or_default();
        let (status, error) = if let Some((status, error)) = &settings.status {
            (status.clone(), *error)
        } else if let Some(request) = &settings.pending {
            (
                if request.wallet {
                    "Waiting for wallet approval…"
                } else {
                    "Checking approval…"
                }
                .to_string(),
                false,
            )
        } else if !settings.config.enabled {
            ("Disabled".to_string(), false)
        } else if !saved {
            (
                "Unsaved changes; orders use the saved builder".to_string(),
                false,
            )
        } else if let Err(error) = &account {
            (error.clone(), true)
        } else if let Some((request, max_fee)) = &settings.approval
            && account.as_ref().ok() == Some(&request.account)
            && *max_fee >= request.builder.fee_tenths_bps()
        {
            ("Approval verified for this account".to_string(), false)
        } else {
            (
                "Enabled · approval checked before trading".to_string(),
                false,
            )
        };
        column![
            row![text("Builder Code").size(14).width(Fill), checkbox(settings.config.enabled)
                .label("Enabled").text_size(12).size(12).on_toggle(Message::SetBuilderCodeEnabled)]
                .align_y(iced::Alignment::Center),
            text_input("Builder wallet address (0x…)", &settings.address_input)
                .style(helpers::text_input_style).size(12).padding(6)
                .on_input(|value| Message::BuilderAddressChanged(value.into())),
            row![text_input("Fee in bps (0.1–10)", &settings.fee_input)
                .style(helpers::text_input_style).size(12).padding(6)
                .on_input(|value| Message::BuilderFeeChanged(value.into()))
                .on_submit(Message::SaveBuilderCode),
                text(fee_preview).size(12),
                button(text("Save").size(12)).padding([6,12]).on_press(Message::SaveBuilderCode)]
                .spacing(8).align_y(iced::Alignment::Center),
            row![button(text("Check approval").size(12)).padding([6,12])
                    .on_press_maybe(available.then_some(Message::CheckBuilderApproval)),
                button(text("Approve in wallet").size(12)).padding([6,12])
                    .on_press_maybe(available.then_some(Message::ApproveBuilderInWallet))].spacing(8),
            text(status).size(11).color(if error { theme.palette().danger } else { theme.extended_palette().background.weak.text }),
            text("Adds the selected fee to perpetual trades and spot/outcome sells. Approval is per account.").size(11)
                .color(theme.extended_palette().background.weak.text),
            text("New orders use saved settings. Running Chase/TWAPs keep their original routing. New Chase orders, order moves and subaccounts require builder codes to be disabled.").size(11)
                .color(theme.extended_palette().background.weak.text),
            text("Disabling stops use on new orders; it does not revoke wallet approval.").size(11)
                .color(theme.extended_palette().background.weak.text),
        ].spacing(8).into()
    }
}
