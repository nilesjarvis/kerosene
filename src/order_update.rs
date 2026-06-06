use crate::app_state::TradingTerminal;
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

pub(crate) use nuke::nuke_confirmation_is_armed;

impl TradingTerminal {
    pub(crate) fn update_order(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OrderPriceChanged(value) => self.handle_order_price_changed(value),
            Message::SetMidPrice => self.handle_set_mid_price(),
            Message::OrderBookPriceSelected { id, price } => {
                return self.handle_order_book_price_selected(id, price);
            }
            Message::OrderQuantityChanged(value) => self.handle_order_quantity_changed(value),
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
            Message::SubmitOrderLeverage => return self.submit_order_leverage_update(),
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
            Message::PlaceBuy | Message::PlaceSell => {
                let is_buy = matches!(message, Message::PlaceBuy);
                return match self.order_kind {
                    crate::signing::OrderKind::Chase => self.start_chase(is_buy),
                    crate::signing::OrderKind::Twap => self.start_twap(is_buy),
                    crate::signing::OrderKind::Market
                    | crate::signing::OrderKind::Limit
                    | crate::signing::OrderKind::LimitIoc => self.execute_order(is_buy),
                };
            }
            Message::OrderResult {
                pending_indicator_id,
                result,
            } => return self.handle_order_result(pending_indicator_id, *result),
            Message::CancelOrder { coin, oid } => return self.execute_cancel(&coin, oid),
            Message::CancelResult {
                pending_indicator_id,
                result,
            } => return self.handle_cancel_result(pending_indicator_id, *result),
            Message::ToggleCloseMenu(coin) => self.toggle_close_menu(coin),
            Message::ClosePosition {
                coin,
                fraction,
                use_market,
            } => {
                self.close_menu_coin = None;
                return self.execute_close_position(&coin, fraction, use_market);
            }
            Message::ClosePositionResult(result) => {
                return self.handle_close_position_result(*result);
            }
            Message::NukePositions => return self.handle_nuke_positions(),
            Message::NukeResult(result) => return self.handle_nuke_result(*result),
            Message::StartChase(is_buy) => return self.start_chase(is_buy),
            Message::StopChase => return self.stop_chase(),
            Message::StopChaseById(chase_id) => return self.stop_chase_by_id(chase_id),
            Message::StopAllAdvancedOrders => {
                let chase_task = self.stop_all_chases();
                let twap_task = self.stop_all_twaps();
                return Task::batch([chase_task, twap_task]);
            }
            Message::TwapDurationChanged(value) => self.handle_twap_duration_changed(value),
            Message::TwapSlicesChanged(value) => self.handle_twap_slices_changed(value),
            Message::TwapMinPriceChanged(value) => self.handle_twap_min_price_changed(value),
            Message::TwapMaxPriceChanged(value) => self.handle_twap_max_price_changed(value),
            Message::TwapRandomizeToggled(value) => self.handle_twap_randomize_toggled(value),
            Message::StartTwap(is_buy) => return self.start_twap(is_buy),
            Message::StopTwap(twap_id) => return self.stop_twap(twap_id),
            Message::TwapTick => return self.handle_twap_tick(),
            Message::TwapBookUpdate {
                twap_id,
                coin,
                book,
            } => return self.handle_twap_book_update(twap_id, coin, book),
            Message::TwapSliceResult { twap_id, result } => {
                return self.handle_twap_slice_result(twap_id, *result);
            }
            Message::TwapUnexpectedCancelResult {
                twap_id,
                oid,
                cloid,
                result,
            } => return self.handle_twap_unexpected_cancel_result(twap_id, oid, cloid, *result),
            Message::TwapOrderStatusLoaded {
                twap_id,
                cloid,
                result,
            } => return self.handle_twap_order_status_result(twap_id, cloid, *result),
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
                book,
            } => return self.handle_chase_book_update(chase_id, coin, book),
            Message::ChaseRepriceTick => return self.handle_chase_reprice_tick(),
            Message::ChasePlaceResult { chase_id, result } => {
                return self.handle_chase_place_result(chase_id, *result);
            }
            Message::ChaseModifyResult {
                chase_id,
                oid,
                result,
            } => return self.handle_chase_modify_result(chase_id, oid, *result),
            Message::ChaseCancelResult {
                chase_id,
                oid,
                result,
            } => return self.handle_chase_cancel_result(chase_id, oid, *result),
            Message::ChaseOrderStatusLoaded {
                chase_id,
                cloid,
                result,
            } => return self.handle_chase_order_status_result(chase_id, cloid, *result),
            Message::ChaseOrderOidStatusLoaded {
                chase_id,
                oid,
                result,
            } => return self.handle_chase_order_oid_status_result(chase_id, oid, *result),
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
            Message::QuickOrderQtyChanged(id, qty) => self.handle_quick_order_qty_changed(id, qty),
            Message::QuickOrderPercentageChanged(id, value) => {
                self.handle_quick_order_percentage_changed(id, value)
            }
            Message::QuickOrderToggleDenomination(id) => {
                self.handle_quick_order_toggle_denomination(id)
            }
            Message::QuickOrderToggleType(id) => self.handle_quick_order_toggle_type(id),
            Message::CloseQuickOrder(id) => self.handle_close_quick_order(id),
            Message::SubmitQuickOrder(chart_id, is_buy) => {
                return self.handle_submit_quick_order(chart_id, is_buy);
            }
            Message::QuickOrderResult {
                pending_indicator_id,
                result,
            } => return self.handle_quick_order_result(pending_indicator_id, *result),
            Message::SubmitHudOrder(request) => return self.handle_submit_hud_order(request),
            Message::HudOrderResult {
                pending_indicator_id,
                result,
            } => return self.handle_hud_order_result(pending_indicator_id, *result),
            Message::EscapePressed => self.clear_transient_order_ui(),
            Message::MoveOrderDragStarted { oid } => {
                self.active_move_order_drag = Some(oid);
            }
            Message::MoveOrder { oid, new_price } => {
                self.active_move_order_drag = None;
                return self.handle_move_order(oid, new_price);
            }
            Message::MoveOrderModifyResult {
                oid,
                pending_indicator_id,
                result,
            } => {
                return self.handle_move_order_modify_result(oid, pending_indicator_id, *result);
            }
            Message::ChaseRestingOrder {
                coin,
                oid,
                is_buy,
                sz,
                limit_px,
                reduce_only,
            } => {
                return self.handle_chase_resting_order(
                    coin,
                    oid,
                    is_buy,
                    sz,
                    limit_px,
                    reduce_only,
                );
            }
            _ => {}
        }

        Task::none()
    }
}
