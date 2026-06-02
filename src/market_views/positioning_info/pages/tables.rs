use crate::app_state::TradingTerminal;
use crate::hyperdash_api::{PerpDeltas, TickerPositions};
use crate::message::Message;
use crate::positioning_state::{POSITIONING_CHANGE_ROW_LIMIT, PositioningInfoInstance};

use super::super::columns::{PositioningChangeColumns, PositioningInfoColumns};
use super::super::metrics::{positioning_live_mark, sorted_change_rows};
use super::super::table::{
    positioning_change_row, positioning_change_table_header, positioning_position_row,
    positioning_table_header,
};
use iced::widget::{Column, container, rule, scrollable, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Positioning Information Tables
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_positioning_info_table(
        &self,
        data: &TickerPositions,
        instance: &PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let columns = PositioningInfoColumns::for_width(available_width);
        let live_mark = positioning_live_mark(instance, TradingTerminal::now_ms());
        let denomination = self.display_denomination_context();
        let hovered_wallet_action_key = self.hovered_wallet_address_actions.as_deref();
        let mut rows = Column::new()
            .spacing(3)
            .push(positioning_table_header(
                instance.id,
                instance.sort_field,
                instance.sort_direction,
                columns,
                theme,
            ))
            .push(rule::horizontal(1));

        if data.positions.is_empty() {
            rows = rows.push(
                container(
                    text("No positions found")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .padding([8, 0]),
            );
        } else {
            for position in &data.positions {
                rows = rows.push(positioning_position_row(
                    instance.id,
                    position,
                    self.wallet_display(&position.address),
                    columns,
                    hovered_wallet_action_key,
                    theme,
                    live_mark,
                    &denomination,
                ));
            }
        }

        scrollable(rows).width(Fill).height(Fill).into()
    }

    pub(super) fn view_positioning_info_change_table(
        &self,
        data: &PerpDeltas,
        instance: &PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let columns = PositioningChangeColumns::for_width(available_width);
        let live_mark = positioning_live_mark(instance, TradingTerminal::now_ms());
        let denomination = self.display_denomination_context();
        let hovered_wallet_action_key = self.hovered_wallet_address_actions.as_deref();
        let sorted = sorted_change_rows(
            &data.deltas,
            instance.change_sort_field,
            instance.change_sort_direction,
            live_mark,
        );
        let mut rows = Column::new()
            .spacing(3)
            .push(positioning_change_table_header(
                instance.id,
                instance.change_sort_field,
                instance.change_sort_direction,
                columns,
                theme,
                &denomination,
            ))
            .push(rule::horizontal(1));

        if sorted.is_empty() {
            rows = rows.push(
                container(
                    text("No changes found")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .padding([8, 0]),
            );
        } else {
            for entry in sorted.into_iter().take(POSITIONING_CHANGE_ROW_LIMIT) {
                rows = rows.push(positioning_change_row(
                    instance.id,
                    entry,
                    self.wallet_display(&entry.address),
                    columns,
                    hovered_wallet_action_key,
                    theme,
                    live_mark,
                    &denomination,
                ));
            }
        }

        scrollable(rows).width(Fill).height(Fill).into()
    }
}
