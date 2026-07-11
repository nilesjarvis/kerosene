use crate::app_state::TradingTerminal;
use crate::chart_state::ChartSurfaceId;
use crate::message::Message;

use iced::Task;

mod chase;
mod form;
mod hud;
mod leverage;
mod move_order;
mod nuke;
mod outcome;
mod presets;
mod quick_order;
mod results;

use quick_order::QuickOrderOpenRequest;

pub(crate) use form::OrderQuantityProvenance;
pub(crate) use nuke::{NukeConfirmation, nuke_confirmation_is_armed};
pub(crate) use results::{
    ExecutionOutcomeKind, PendingCancelStatusRequest, PendingMoveStatusRequest,
    PendingOneShotStatusRequest, classify_execution_result,
};

impl TradingTerminal {
    pub(crate) fn update_order(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OrderPriceChanged(value) => {
                self.handle_order_price_changed(value.into_string())
            }
            Message::SetMidPrice => self.handle_set_mid_price(),
            Message::OrderBookPriceSelected { id, price } => {
                return self.handle_order_book_price_selected(id, price);
            }
            Message::OrderQuantityChanged(value) => {
                self.handle_order_quantity_changed(value.into_string())
            }
            Message::ToggleOrderDenomination => self.handle_toggle_order_denomination(),
            Message::OrderPercentageChanged(value) => self.handle_order_percentage_changed(value),
            Message::PrefillOutcomeSell(balance_coin) => {
                return self.handle_prefill_outcome_sell(balance_coin);
            }
            Message::SetOrderKind(kind) => self.handle_set_order_kind(kind),
            Message::ToggleReduceOnly => self.handle_toggle_reduce_only(),
            Message::ToggleOrderLeverageDropdown => self.handle_toggle_order_leverage_dropdown(),
            Message::OrderLeverageInputChanged(value) => {
                self.handle_order_leverage_input_changed(value)
            }
            Message::SetOrderLeverageCross(is_cross) => {
                self.handle_set_order_leverage_cross(is_cross)
            }
            Message::SubmitOrderLeverage(snapshot) => {
                return self.submit_order_leverage_update(snapshot);
            }
            Message::OrderLeverageResult { context, result } => {
                return self.handle_order_leverage_result(context, *result);
            }
            Message::TogglePresetsMenu => self.handle_toggle_presets_menu(),
            Message::TogglePresetCurrency => self.handle_toggle_preset_currency(),
            Message::TogglePresetEditMode => self.handle_toggle_preset_edit_mode(),
            Message::EditPresetStart(kind, idx, current_size_str) => {
                self.handle_edit_preset_start(kind, idx, current_size_str)
            }
            Message::EditPresetChanged(new_text) => self.handle_edit_preset_changed(new_text),
            Message::EditPresetSave(kind, idx) => self.handle_edit_preset_save(kind, idx),
            Message::ExecutePreset(kind, preset, is_buy) => {
                return self.handle_execute_preset(kind, preset, is_buy);
            }
            Message::DismissOrderStatus => self.handle_dismiss_order_status(),
            Message::PlaceOrder { is_buy, snapshot } => {
                return self.execute_order_from_snapshot(is_buy, snapshot);
            }
            Message::OrderResult {
                pending_indicator_id,
                context,
                result,
            } => return self.handle_order_result(pending_indicator_id, context, *result),
            Message::CancelOrder { coin, oid } => {
                return self.execute_cancel(&coin, oid.into_u64());
            }
            Message::CancelResult {
                account_address,
                pending_indicator_id,
                result,
            } => {
                return self.handle_cancel_result(
                    account_address.into_string(),
                    pending_indicator_id,
                    *result,
                );
            }
            Message::CancelOrderStatusLoaded {
                account_address,
                oid,
                symbol,
                result,
            } => {
                return self.handle_cancel_order_status_result(
                    account_address.into_string(),
                    oid.into_u64(),
                    symbol,
                    *result,
                );
            }
            Message::ToggleCloseMenu(coin) => self.toggle_close_menu(coin),
            Message::ClosePosition {
                coin,
                fraction,
                use_market,
            } => {
                self.close_menu_coin = None;
                return self.execute_close_position(&coin, fraction, use_market);
            }
            Message::ClosePositionResult {
                pending_indicator_id,
                context,
                result,
            } => {
                return self.handle_close_position_result(pending_indicator_id, context, *result);
            }
            Message::NukePositions => return self.handle_nuke_positions(),
            Message::NukeResult {
                execution_id,
                context,
                result,
            } => {
                return self.handle_nuke_result(execution_id, context, *result);
            }
            Message::NukePlacementStatusLoaded {
                execution_id,
                context,
                result,
            } => {
                return self.handle_nuke_placement_status_result(execution_id, context, *result);
            }
            Message::OneShotPlacementStatusLoaded {
                request_id,
                context,
                result,
            } => {
                return self.handle_one_shot_placement_status_result(request_id, context, *result);
            }
            Message::StartChase { is_buy, snapshot } => {
                return self.start_chase_from_snapshot(is_buy, snapshot);
            }
            Message::StopChase => return self.stop_chase(),
            Message::StopChaseById(chase_id) => return self.stop_chase_by_id(chase_id),
            Message::StopAllAdvancedOrders => {
                let chase_task = self.stop_all_chases();
                let twap_task = self.stop_all_twaps();
                return Task::batch([chase_task, twap_task]);
            }
            Message::TwapDurationChanged(value) => {
                self.handle_twap_duration_changed(value.into_string())
            }
            Message::TwapSlicesChanged(value) => {
                self.handle_twap_slices_changed(value.into_string())
            }
            Message::TwapMinPriceChanged(value) => {
                self.handle_twap_min_price_changed(value.into_string())
            }
            Message::TwapMaxPriceChanged(value) => {
                self.handle_twap_max_price_changed(value.into_string())
            }
            Message::TwapRandomizeToggled(value) => self.handle_twap_randomize_toggled(value),
            Message::StartTwap { is_buy, snapshot } => {
                return self.start_twap_from_snapshot(is_buy, snapshot);
            }
            Message::StopTwap(twap_id) => return self.stop_twap(twap_id),
            Message::TwapTick => return self.handle_twap_tick(),
            Message::TwapBookUpdate {
                twap_id,
                coin,
                sigfigs,
                source_context,
                book,
            } => {
                return self.handle_twap_book_update(twap_id, coin, sigfigs, source_context, book);
            }
            Message::TwapBookLagged {
                twap_id,
                coin,
                sigfigs,
                source_context,
                skipped,
            } => {
                return self.handle_twap_book_lagged(
                    twap_id,
                    coin,
                    sigfigs,
                    source_context,
                    skipped,
                );
            }
            Message::TwapSliceResult {
                twap_id,
                slice_index,
                retry_count,
                result,
            } => {
                return self.handle_twap_slice_result(twap_id, slice_index, retry_count, *result);
            }
            Message::TwapUnexpectedCancelResult {
                twap_id,
                oid,
                cloid,
                result,
            } => {
                return self.handle_twap_unexpected_cancel_result(
                    twap_id,
                    oid.map(|oid| oid.into_u64()),
                    cloid.map(|cloid| cloid.into_string()),
                    *result,
                );
            }
            Message::TwapUnexpectedCancelRetryDue {
                twap_id,
                oid,
                cloid,
                attempt,
            } => {
                return self.handle_twap_unexpected_cancel_retry_due(
                    twap_id,
                    oid.map(|oid| oid.into_u64()),
                    cloid.map(|cloid| cloid.into_string()),
                    attempt,
                );
            }
            Message::TwapOrderStatusLoaded {
                twap_id,
                cloid,
                result,
            } => {
                return self.handle_twap_order_status_result(twap_id, cloid.into_string(), *result);
            }
            Message::OpenTwapDetails(twap_id) => return self.open_twap_details(twap_id),
            Message::OpenAdvancedOrderHistory(entry_id) => {
                return self.open_advanced_order_history(entry_id);
            }
            Message::ChaseInitialBookLoaded { chase_id, result } => {
                return self.handle_chase_initial_book_loaded(chase_id, *result);
            }
            Message::ChaseBookUpdate {
                chase_id,
                coin,
                sigfigs,
                source_context,
                book,
            } => {
                return self.handle_chase_book_update(
                    chase_id,
                    coin,
                    sigfigs,
                    source_context,
                    book,
                );
            }
            Message::ChaseBookLagged {
                chase_id,
                coin,
                sigfigs,
                source_context,
                skipped,
            } => {
                return self.handle_chase_book_lagged(
                    chase_id,
                    coin,
                    sigfigs,
                    source_context,
                    skipped,
                );
            }
            Message::ChaseRepriceTick => return self.handle_chase_reprice_tick(),
            Message::ChasePlaceResult {
                chase_id,
                place_attempt,
                result,
            } => {
                return self.handle_chase_place_result(chase_id, place_attempt, *result);
            }
            Message::ChaseModifyResult {
                chase_id,
                oid,
                reprice_count,
                result,
            } => {
                return self.handle_chase_modify_result(
                    chase_id,
                    oid.into_u64(),
                    reprice_count,
                    *result,
                );
            }
            Message::ChaseCancelResult {
                chase_id,
                oid,
                result,
            } => return self.handle_chase_cancel_result(chase_id, oid.into_u64(), *result),
            Message::ChaseOrderStatusLoaded {
                chase_id,
                cloid,
                result,
            } => {
                return self.handle_chase_order_status_result(
                    chase_id,
                    cloid.into_string(),
                    *result,
                );
            }
            Message::ChaseOrderOidStatusLoaded {
                chase_id,
                oid,
                result,
            } => {
                return self.handle_chase_order_oid_status_result(
                    chase_id,
                    oid.into_u64(),
                    *result,
                );
            }
            Message::OpenQuickOrder(
                chart_id,
                surface_id,
                price,
                click_x,
                click_y,
                chart_w,
                chart_h,
            ) => {
                return self.handle_open_quick_order(QuickOrderOpenRequest {
                    chart_id,
                    surface_id,
                    price,
                    click_x,
                    click_y,
                    chart_w,
                    chart_h,
                });
            }
            Message::QuickOrderQtyChanged(id, qty) => {
                self.handle_quick_order_qty_changed(id, qty.into_string())
            }
            Message::QuickOrderPercentageChanged(id, value) => {
                self.handle_quick_order_percentage_changed(id, value)
            }
            Message::QuickOrderToggleDenomination(id) => {
                self.handle_quick_order_toggle_denomination(id)
            }
            Message::QuickOrderToggleType(id) => self.handle_quick_order_toggle_type(id),
            Message::CloseQuickOrder(id) => self.handle_close_quick_order(id),
            Message::SubmitQuickOrder {
                chart_id,
                is_buy,
                snapshot,
            } => {
                return self.handle_submit_quick_order_from_snapshot(chart_id, is_buy, snapshot);
            }
            Message::QuickOrderResult {
                pending_indicator_id,
                context,
                recovery,
                result,
            } => {
                return self.handle_quick_order_result(
                    pending_indicator_id,
                    context,
                    recovery,
                    *result,
                );
            }
            Message::SubmitHudOrder(request) => return self.handle_submit_hud_order(request),
            Message::HudOrderResult {
                pending_indicator_id,
                inflight_id,
                context,
                result,
            } => {
                return self.handle_hud_order_result(
                    pending_indicator_id,
                    inflight_id,
                    context,
                    *result,
                );
            }
            Message::EscapePressed(window_id) => self.handle_order_escape_pressed(window_id),
            Message::MoveOrderDragStarted { coin, oid } => {
                self.active_move_order_drag = Some(crate::order_execution::MoveOrderKey::new(
                    coin,
                    oid.into_u64(),
                ));
            }
            Message::MoveOrder {
                coin,
                oid,
                new_price,
            } => {
                self.active_move_order_drag = None;
                return self.handle_move_order(coin, oid.into_u64(), new_price);
            }
            Message::MoveOrderModifyResult {
                account_address,
                coin,
                oid,
                pending_indicator_id,
                result,
            } => {
                return self.handle_move_order_modify_result(
                    account_address.into_string(),
                    coin,
                    oid.into_u64(),
                    pending_indicator_id,
                    *result,
                );
            }
            Message::MoveOrderStatusLoaded {
                account_address,
                coin,
                oid,
                result,
            } => {
                return self.handle_move_order_status_result(
                    account_address.into_string(),
                    coin,
                    oid.into_u64(),
                    *result,
                );
            }
            Message::ChaseRestingOrder { coin, oid } => {
                return self.handle_chase_resting_order(coin, oid.into_u64());
            }
            // Every message routed to `UpdateRoute::Order` has an explicit arm
            // above, so this is unreachable today. If a future order message is
            // routed here without a handler, fail loudly in debug/test builds
            // instead of silently dropping it; release stays a benign no-op.
            #[cfg(debug_assertions)]
            other => {
                unreachable!("order message routed to update_order without a handler: {other:?}")
            }
            #[cfg(not(debug_assertions))]
            _ => {}
        }

        Task::none()
    }

    fn handle_order_escape_pressed(&mut self, window_id: iced::window::Id) {
        if self
            .main_window_id
            .is_none_or(|main_id| main_id == window_id)
        {
            self.clear_transient_order_ui();
            return;
        }

        let Some(chart_id) = self
            .detached_chart_windows
            .get(&window_id)
            .map(|state| state.chart_id)
        else {
            return;
        };

        self.clear_chart_surface_state(chart_id, ChartSurfaceId::Detached(window_id));
        if let Some(instance) = self.charts.get_mut(&chart_id) {
            instance.editor_open = false;
            instance.editor_search_query.clear();
            instance.editor_selected_index = None;
            instance.secondary_editor_open = false;
            instance.secondary_editor_search_query.clear();
            instance.secondary_editor_selected_index = None;
        }
    }
}
