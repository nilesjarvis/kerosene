use super::metrics::{positioning_flow_data, positioning_live_mark};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::positioning_state::PositioningInfoInstance;
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};

use chart::{PositioningFlowChart, PositioningFlowChartRow};
use iced::widget::{Column, Space, canvas as canvas_widget, container, responsive, row, stack};
use iced::{Element, Fill, Length, Theme};

mod chart;

/// Number of top movers visualized as bars. The flow view is a ranking of the
/// largest moves, so a focused cap keeps it scannable instead of endless.
const POSITIONING_FLOW_ROW_LIMIT: usize = 60;

impl TradingTerminal {
    pub(in crate::market_views::positioning_info) fn view_positioning_info_flow(
        &self,
        instance: &PositioningInfoInstance,
        now_ms: u64,
    ) -> Element<'_, Message> {
        let live_mark = positioning_live_mark(instance, now_ms);
        let denomination = self.display_denomination_context();
        let data = match &instance.change_data {
            Some(data) => {
                positioning_flow_data(&data.deltas, live_mark, POSITIONING_FLOW_ROW_LIMIT)
            }
            None => positioning_flow_data(&[], live_mark, POSITIONING_FLOW_ROW_LIMIT),
        };

        // Resolve trader labels against the address book before handing pure
        // data to the canvas program.
        let mut chart = PositioningFlowChart::new(&data, &denomination);
        for (row, source) in chart.rows.iter_mut().zip(data.rows.iter()) {
            row.label = self.wallet_display(&source.address).primary;
            row.hover_key = format!("positioning-flow:{}:{}", instance.id, source.address);
        }
        chart.hovered_action_key = self.hovered_wallet_address_actions.clone();

        let height = chart.content_height();
        let theme = self.theme();

        responsive(move |size| {
            let chart_layer: Element<'_, Message> = canvas_widget(chart.clone())
                .width(Fill)
                .height(Length::Fixed(height))
                .into();

            // The interactive label overlay only appears when the canvas shows
            // the label column, so the buttons never collide with the bars.
            if PositioningFlowChart::labels_visible(size.width)
                && let Some(overlay) =
                    build_action_overlay(chart.rows(), chart.hovered_action_key.as_deref(), &theme)
            {
                container(stack![chart_layer, overlay])
                    .width(Fill)
                    .height(Length::Fixed(height))
                    .into()
            } else {
                container(chart_layer)
                    .width(Fill)
                    .height(Length::Fixed(height))
                    .into()
            }
        })
        .into()
    }
}

/// Builds the interactive trader-label overlay aligned to the canvas rows. Each
/// slot shows the resolved label and, on hover, swaps to copy/detach/ghost
/// action segments (the same widget the positioning table uses).
fn build_action_overlay(
    rows: &[PositioningFlowChartRow],
    hovered_key: Option<&str>,
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    if rows.is_empty() {
        return None;
    }

    let label_left = PositioningFlowChart::label_left();
    let label_width = PositioningFlowChart::label_width();
    let row_height = PositioningFlowChart::row_height();
    let row_gap = PositioningFlowChart::row_gap();

    let mut column =
        Column::new().push(Space::new().height(Length::Fixed(PositioningFlowChart::rows_top())));

    for row in rows {
        let cell = wallet_address_action_cell(WalletAddressActionCell {
            address: row.address.clone(),
            label: row.label.clone(),
            tooltip_label: format!("Copy {}", row.address),
            hover_key: row.hover_key.clone(),
            hovered_key,
            width: label_width,
            text_size: 11,
            text_color: theme.palette().text,
        });

        let slot = row![
            Space::new().width(Length::Fixed(label_left)),
            cell,
            Space::new().width(Fill),
        ]
        .height(Length::Fixed(row_height))
        .align_y(iced::Alignment::Center);

        column = column
            .push(slot)
            .push(Space::new().height(Length::Fixed(row_gap)));
    }

    Some(column.width(Fill).into())
}
