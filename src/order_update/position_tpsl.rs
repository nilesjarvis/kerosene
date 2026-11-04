use crate::account_state::position_tpsl::{PositionTpslForm, PositionTpslSubmission, TpslOutcome};
use crate::api::fetch_order_status_by_cloid;
use crate::app_state::TradingTerminal;
use crate::canvas_state::WorkspaceId;
use crate::message::Message;
use crate::order_execution::{position_tpsl_outcomes, submit_position_tpsl_orders};
use crate::signing::ExchangeResponse;
use iced::Task;

impl TradingTerminal {
    pub(crate) fn sync_position_tpsl_order_status(&mut self) {
        if let Some(form) = &mut self.position_tpsl.form
            && form.visible
        {
            form.status = self.order_status.clone();
        }
    }

    pub(crate) fn open_position_tpsl(
        &mut self,
        workspace: WorkspaceId,
        coin: String,
    ) -> Task<Message> {
        if self.position_tpsl.pending.is_some() {
            if let Some(form) = &mut self.position_tpsl.form {
                form.visible = true;
                form.workspace = workspace;
            }
            return iced::widget::operation::focus("position_tpsl_tp");
        }
        let Some((address, data)) = self.connected_order_account_snapshot() else {
            return self.refresh_account_data();
        };
        if !self.is_perp_coin(&coin) || !self.has_active_committed_agent_key() {
            return Task::none();
        }
        let Some(size) = data
            .clearinghouse
            .asset_positions
            .iter()
            .find(|ap| ap.position.coin == coin)
            .and_then(|ap| ap.position.szi.parse::<f64>().ok())
            .filter(|size| size.is_finite() && size.abs() > 1e-12)
        else {
            return Task::none();
        };
        self.position_tpsl.form = Some(PositionTpslForm {
            workspace,
            visible: true,
            account_address: address,
            coin,
            is_long: size > 0.0,
            take_profit: String::new(),
            stop_loss: String::new(),
            status: None,
        });
        self.close_menu_coin = None;
        Task::batch([
            self.refresh_account_data(),
            iced::widget::operation::focus("position_tpsl_tp"),
        ])
    }

    fn position_tpsl_status(&mut self, status: String, error: bool) {
        if let Some(form) = &mut self.position_tpsl.form {
            form.status = Some((status.clone(), error));
        }
        self.set_order_status(status, error);
    }

    pub(crate) fn submit_position_tpsl(&mut self) -> Task<Message> {
        if self.reject_if_pending_trading_request("setting TP/SL") {
            if let Some(form) = &mut self.position_tpsl.form {
                form.status = self.order_status.clone();
            }
            return Task::none();
        }
        let Some((key, address)) = self.order_signing_context() else {
            if let Some(form) = &mut self.position_tpsl.form {
                form.status = self.order_status.clone();
            }
            return Task::none();
        };
        let orders = match self.prepare_position_tpsl() {
            Ok(orders) => orders,
            Err(error) => {
                self.position_tpsl_status(error, true);
                return self.refresh_account_data();
            }
        };
        let Some(form) = &self.position_tpsl.form else {
            return Task::none();
        };
        let coin = form.coin.clone();
        self.position_tpsl.next_id = self.position_tpsl.next_id.wrapping_add(1);
        let id = self.position_tpsl.next_id;
        self.position_tpsl.pending = Some(PositionTpslSubmission {
            id,
            account_address: address,
            outcomes: vec![TpslOutcome::Unknown; orders.len()],
            orders: orders.clone(),
            in_flight: true,
        });
        self.position_tpsl_status("Setting TP/SL…".into(), false);
        // Trigger orders do not project an immediate position-size change.
        Task::perform(
            submit_position_tpsl_orders(key, coin, orders),
            move |result| Message::PositionTpslResult {
                id,
                result: Box::new(result),
            },
        )
    }

    fn current_position_tpsl_submission(&self, id: u64) -> bool {
        self.position_tpsl.pending.as_ref().is_some_and(|pending| {
            pending.id == id && self.connected_order_account_matches(&pending.account_address)
        })
    }

    pub(crate) fn handle_position_tpsl_result(
        &mut self,
        id: u64,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        if !self.current_position_tpsl_submission(id) {
            return Task::none();
        }
        let Some(pending) = &mut self.position_tpsl.pending else {
            return Task::none();
        };
        pending.outcomes = position_tpsl_outcomes(result, pending.orders.len());
        pending.in_flight = false;
        if pending.outcomes.contains(&TpslOutcome::Unknown) {
            return Task::batch([self.check_position_tpsl(), self.refresh_account_data()]);
        }
        self.finish_position_tpsl()
    }

    pub(crate) fn check_position_tpsl(&mut self) -> Task<Message> {
        let Some(pending) = &mut self.position_tpsl.pending else {
            return Task::none();
        };
        if pending.in_flight {
            return Task::none();
        }
        pending.in_flight = true;
        let id = pending.id;
        let address = pending.account_address.clone();
        let orders = pending.orders.clone();
        let mut outcomes = pending.outcomes.clone();
        self.position_tpsl_status("Checking TP/SL status…".into(), false);
        Task::perform(
            async move {
                for (order, outcome) in orders.iter().zip(outcomes.iter_mut()) {
                    if *outcome != TpslOutcome::Unknown {
                        continue;
                    }
                    if let Ok(status) =
                        fetch_order_status_by_cloid(address.clone(), order.cloid.clone()).await
                    {
                        if status.is_open() || status.is_filled() {
                            *outcome = TpslOutcome::Accepted;
                        } else if status.is_no_fill_terminal() {
                            *outcome = TpslOutcome::Rejected("Order is no longer active".into());
                        }
                    }
                }
                outcomes
            },
            move |outcomes| Message::PositionTpslChecked { id, outcomes },
        )
    }

    pub(crate) fn handle_position_tpsl_checked(
        &mut self,
        id: u64,
        outcomes: Vec<TpslOutcome>,
    ) -> Task<Message> {
        if !self.current_position_tpsl_submission(id) {
            return Task::none();
        }
        let Some(pending) = &mut self.position_tpsl.pending else {
            return Task::none();
        };
        if outcomes.len() != pending.orders.len() {
            return Task::none();
        }
        pending.in_flight = false;
        pending.outcomes = outcomes;
        if pending.outcomes.contains(&TpslOutcome::Unknown) {
            self.position_tpsl_status(
                "TP/SL status is uncertain. Check status before submitting again.".into(),
                true,
            );
            return self.refresh_account_data();
        }
        self.finish_position_tpsl()
    }

    fn finish_position_tpsl(&mut self) -> Task<Message> {
        let Some(pending) = self.position_tpsl.pending.take() else {
            return Task::none();
        };
        let mut labels = Vec::new();
        let mut has_error = false;
        for (order, outcome) in pending.orders.iter().zip(&pending.outcomes) {
            match outcome {
                TpslOutcome::Accepted => {
                    if let Some(form) = &mut self.position_tpsl.form {
                        form.input_mut(order.kind).clear();
                    }
                    labels.push(format!("{} accepted", order.kind.label()));
                }
                TpslOutcome::Rejected(error) => {
                    has_error = true;
                    labels.push(format!("{}: {error}", order.kind.label()));
                }
                TpslOutcome::Unknown => {
                    unreachable!("unresolved TP/SL keeps its pending submission")
                }
            }
        }
        self.position_tpsl_status(labels.join(" · "), has_error);
        self.refresh_account_data()
    }
}
