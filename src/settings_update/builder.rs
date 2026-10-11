use crate::app_state::TradingTerminal;
use crate::builder_codes::{BuilderCodeConfig, BuilderFee, fetch_builder_approval};
use crate::message::Message;
use crate::settings_state::BuilderApprovalRequest;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_builder_codes(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::BuilderAddressChanged(value) => {
                self.builder_codes.address_input = value.into_string().into();
                self.builder_codes.status = None;
            }
            Message::BuilderFeeChanged(value) => {
                self.builder_codes.fee_input = value.into_string();
                self.builder_codes.status = None;
            }
            Message::SaveBuilderCode => {
                let builder = match BuilderFee::from_inputs(
                    &self.builder_codes.address_input,
                    &self.builder_codes.fee_input,
                ) {
                    Ok(builder) => builder,
                    Err(error) => {
                        self.builder_codes.status = Some((error, true));
                        return Task::none();
                    }
                };
                self.builder_codes.config = BuilderCodeConfig {
                    enabled: self.builder_codes.config.enabled,
                    address: builder.address().to_string(),
                    fee_tenths_bps: builder.fee_tenths_bps(),
                };
                self.builder_codes.address_input = builder.address().to_string().into();
                self.builder_codes.fee_input = builder.fee_bps_text();
                self.builder_codes.invalidate();
                self.persist_config();
                return self.refresh_builder_approval();
            }
            Message::SetBuilderCodeEnabled(enabled) => {
                if enabled && !self.builder_codes.draft_is_saved() {
                    self.builder_codes.status =
                        Some(("Save a valid builder address and fee first".into(), true));
                    return Task::none();
                }
                self.builder_codes.config.enabled = enabled;
                self.builder_codes.invalidate();
                self.persist_config();
                if enabled {
                    return self.refresh_builder_approval();
                }
            }
            Message::CheckBuilderApproval => return self.refresh_builder_approval(),
            Message::ApproveBuilderInWallet => return self.start_builder_wallet_approval(),
            Message::BuilderApprovalChecked(request, result) => {
                if !self.builder_approval_request_is_current(&request) {
                    self.clear_stale_builder_request(&request);
                    return Task::none();
                }
                self.builder_codes.pending = None;
                match result {
                    Ok(max_fee) => {
                        self.builder_codes.approval = Some((request.clone(), max_fee));
                        self.builder_codes.status = if max_fee < request.builder.fee_tenths_bps() {
                            Some(("Approval required for this account and fee".into(), true))
                        } else {
                            None
                        };
                    }
                    Err(error) => {
                        self.builder_codes.approval = None;
                        self.builder_codes.status = Some((error, true));
                    }
                }
            }
            Message::BuilderWalletApprovalFinished(request, result) => {
                if !self.builder_approval_request_is_current(&request) {
                    self.clear_stale_builder_request(&request);
                    return Task::none();
                }
                self.builder_codes.pending = None;
                match result {
                    Ok(()) => return self.refresh_builder_approval(),
                    Err(error) => self.builder_codes.status = Some((error, true)),
                }
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn builder_approval_account(&self) -> Result<String, String> {
        let profile = self
            .accounts
            .get(self.active_account_index)
            .ok_or_else(|| "Connect a trading account to check builder approval".to_string())?;
        if self.active_account_is_ghost() {
            return Err("Watch-only accounts cannot configure trading approvals".into());
        }
        if profile.master_address.is_some() {
            return Err("Builder codes are not yet supported for subaccounts".into());
        }
        if !self.connected_order_account_matches(&profile.wallet_address)
            || !self.active_wallet_context_matches_connected_account(&profile.wallet_address)
        {
            return Err("Connect the selected trading account first".into());
        }
        crate::builder_codes::normalize_builder_address(&profile.wallet_address)
            .map_err(|_| "The selected account address is invalid".to_string())
    }

    fn new_builder_approval_request(
        &mut self,
        wallet: bool,
    ) -> Result<BuilderApprovalRequest, String> {
        if self.builder_codes.pending.is_some() {
            return Err("A builder approval request is already in progress".into());
        }
        if !self.builder_codes.draft_is_saved() {
            return Err("Save the builder address and fee first".into());
        }
        let account = self.builder_approval_account()?;
        let builder = self.builder_codes.saved_builder()?;
        self.builder_codes.generation = self.builder_codes.generation.wrapping_add(1);
        let request = BuilderApprovalRequest {
            generation: self.builder_codes.generation,
            account,
            builder,
            wallet,
        };
        self.builder_codes.pending = Some(request.clone());
        self.builder_codes.approval = None;
        self.builder_codes.status = None;
        Ok(request)
    }

    fn builder_approval_request_is_current(&self, request: &BuilderApprovalRequest) -> bool {
        self.builder_codes.pending.as_ref() == Some(request)
            && self.builder_codes.generation == request.generation
            && self.builder_codes.saved_builder().ok().as_ref() == Some(&request.builder)
            && self.builder_approval_account().ok().as_deref() == Some(request.account.as_str())
    }

    fn clear_stale_builder_request(&mut self, request: &BuilderApprovalRequest) {
        // Editing the wallet address can change the trading context without a
        // connect/disconnect event. Retire only this completed request; never
        // clear a newer request or accept its stale approval result.
        if self.builder_codes.pending.as_ref() == Some(request) {
            self.builder_codes.pending = None;
            self.builder_codes.approval = None;
        }
    }

    pub(crate) fn refresh_builder_approval(&mut self) -> Task<Message> {
        let request = match self.new_builder_approval_request(false) {
            Ok(request) => request,
            Err(error) => {
                self.builder_codes.status = Some((error, true));
                return Task::none();
            }
        };
        Task::perform(
            async move {
                let result = fetch_builder_approval(&request.account, &request.builder).await;
                (request, result)
            },
            |(request, result)| Message::BuilderApprovalChecked(request, result),
        )
    }

    fn start_builder_wallet_approval(&mut self) -> Task<Message> {
        let request = match self.new_builder_approval_request(true) {
            Ok(request) => request,
            Err(error) => {
                self.builder_codes.status = Some((error, true));
                return Task::none();
            }
        };
        Task::perform(
            async move {
                let result = crate::builder_codes::approval::approve_in_wallet(
                    request.account.clone(),
                    request.builder.clone(),
                )
                .await;
                (request, result)
            },
            |(request, result)| Message::BuilderWalletApprovalFinished(request, result),
        )
    }
}

#[cfg(test)]
mod tests;
