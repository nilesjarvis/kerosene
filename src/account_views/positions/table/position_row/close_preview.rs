use super::super::sort::PositionRowData;
use crate::account_state::close_preview::{
    CloseEstimate, CloseEstimateError, estimate_position_close,
};
use crate::app_state::TradingTerminal;
use crate::helpers::format_price;
use crate::message::Message;
use iced::widget::{column, container, mouse_area, row, rule, text, tooltip};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_position_close_preview(
        &self,
        cell: Element<'static, Message>,
        data: &PositionRowData<'_>,
        theme: &Theme,
    ) -> Element<'static, Message> {
        if self.hide_pnl {
            return cell;
        }
        let mut content = column![text("Close entire position · Market").size(13)].spacing(5);
        match self.position_close_estimate(data) {
            Ok(estimate) => {
                let denomination = self.display_denomination_context();
                let money = |value| denomination.format_value(value, 2);
                content = content
                    .push(
                        text(money(estimate.net_pnl))
                            .size(16)
                            .font(crate::app_fonts::monospace_font())
                            .color(self.direction_color(theme, estimate.net_pnl)),
                    )
                    .push(text("Estimated PnL after closing fees").size(11))
                    .push(rule::horizontal(1))
                    .push(detail("Average exit", format_price(estimate.average_price)))
                    .push(detail("PnL before closing fees", money(estimate.gross_pnl)))
                    .push(detail("Closing fees", money(estimate.exit_fee)))
                    .push(detail(
                        "Depth consumed",
                        format!("{} price levels · 100% of position", estimate.levels_used),
                    ))
                    .push(
                        text("Only closing fees deducted; excludes funding. Execution may differ.")
                            .size(11)
                            .color(theme.extended_palette().background.weak.text),
                    );
            }
            Err(reason) => {
                content = content.push(text(reason).size(12));
            }
        }
        mouse_area(
            tooltip(
                cell,
                container(content)
                    .max_width(380)
                    .padding(10)
                    .style(container::bordered_box),
                tooltip::Position::Top,
            )
            .gap(6)
            .snap_within_viewport(true),
        )
        .on_enter(Message::PositionClosePreviewHovered(
            data.coin.to_string(),
            true,
        ))
        .on_exit(Message::PositionClosePreviewHovered(
            data.coin.to_string(),
            false,
        ))
        .into()
    }

    fn position_close_estimate(
        &self,
        data: &PositionRowData<'_>,
    ) -> Result<CloseEstimate, &'static str> {
        let (_, account) = self
            .connected_order_account_snapshot()
            .ok_or("Account data unavailable")?;
        let is_spot = self.is_spot_coin(data.coin);
        let preview = &self.position_close_preview;
        // Book reception advances this clock between one-second UI ticks.
        let now_ms = self.status_bar_now_ms.max(preview.now_ms);
        let fresh_account = if is_spot || self.is_outcome_coin(data.coin) {
            account.is_fresh_for_spot_balance_action(now_ms)
                && account.completeness.spot_balances_complete
        } else {
            account.is_fresh_for_position_action(now_ms)
                && account.completeness.positions_actionable
        };
        if self.account_reconciliation_required
            || self
                .optimistic_position_delta_for_symbol(data.coin)
                .is_some()
            || !fresh_account
        {
            return Err("Waiting for current position data");
        }
        let size = data.szi.ok_or("Position size unavailable")?;
        let entry = data.entry_px.ok_or("Position cost basis unavailable")?;
        if preview.coin.as_deref() != Some(data.coin) {
            return Err("Loading live order book…");
        }
        let (_, source, _) = preview.book.as_ref().ok_or("Loading live order book…")?;
        if !self.market_stream_source_is_current(*source) {
            return Err("Waiting for current market data");
        }
        let book = preview
            .fresh_book(self.status_bar_now)
            .ok_or("Order book is stale · waiting for live depth")?;
        let multiplier = match &preview.fee_multiplier {
            Some(Ok(multiplier)) => *multiplier,
            Some(Err(_)) => return Err("Market-specific closing fee estimate unavailable"),
            None => return Err("Loading market fee terms…"),
        };
        if !account.completeness.fees_complete {
            return Err("Account fee rate unavailable");
        }
        let fee_rate = account
            .fee_rates
            .close_taker_rate(is_spot)
            .ok_or("Account fee rate unavailable")?
            * multiplier
            + self
                .builder_fee_rate(is_spot, size < 0.0)
                .ok_or("Builder fee configuration unavailable")?;
        let symbol = self
            .exchange_symbol_for_key(data.coin)
            .ok_or("Market metadata unavailable")?;
        let mid = data.mark_px.ok_or("Current market reference unavailable")?;
        let limit = crate::order_execution::pricing::rounded_market_price(
            mid,
            size < 0.0,
            self.market_slippage_fraction(),
            symbol.sz_decimals,
            is_spot,
        );
        estimate_position_close(book, size, entry, fee_rate, limit).map_err(|error| match error {
            CloseEstimateError::InvalidInputs => "Position or fee data unavailable",
            CloseEstimateError::InvalidBook => "Order book unavailable",
            CloseEstimateError::InsufficientDepth => {
                "Not enough visible depth to estimate a full close"
            }
            CloseEstimateError::SlippageLimit => {
                "Full close exceeds the market order slippage limit"
            }
        })
    }
}

fn detail(label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(label).size(11).width(iced::Fill),
        text(value)
            .size(12)
            .font(crate::app_fonts::monospace_font())
    ]
    .spacing(12)
    .into()
}

#[cfg(test)]
mod tests;
