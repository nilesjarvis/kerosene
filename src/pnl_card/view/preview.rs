use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::super::display_text::pnl_card_render_text;
use super::super::metrics::PnlCardMetrics;
use super::super::model::PnlCardWindowState;
use super::super::style::{
    pnl_card_border_style, pnl_card_detail_band_style, pnl_card_inner_style, pnl_card_palette,
};

use iced::widget::{Column, Space, column, container, row, text};
use iced::{Alignment, Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Preview
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_pnl_card_preview<'a>(
        &'a self,
        state: &'a PnlCardWindowState,
        metrics: PnlCardMetrics,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let pnl_color = self.direction_color(theme, metrics.upnl);
        let card_palette = pnl_card_palette(theme, pnl_color);
        let text_color = card_palette.text;
        let weak_text = card_palette.weak_text;
        let denomination = self.display_denomination_context();
        let render_text = pnl_card_render_text(state, &metrics, &denomination);
        let ticker = render_text.ticker;
        let leverage_display = render_text.leverage_display;

        let mut value_stack = Column::new()
            .spacing(4)
            .push(
                text(render_text.primary_value)
                    .size(38)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
            )
            .push(
                text(render_text.percent_mode_label)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(weak_text),
            );
        if let Some(secondary) = render_text.secondary_value {
            value_stack = value_stack.push(
                text(secondary)
                    .size(18)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
            );
        }

        let details = container(
            column![
                row![
                    card_metric("Lev", leverage_display, weak_text, text_color),
                    card_metric("Entry", render_text.entry_display, weak_text, text_color),
                    card_metric("Exit", render_text.exit_display, weak_text, text_color),
                ]
                .spacing(10),
                text(render_text.context)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(weak_text),
            ]
            .spacing(8),
        )
        .width(Fill)
        .padding([8, 10])
        .style(move |theme: &Theme| pnl_card_detail_band_style(theme, pnl_color));

        let card_content = column![
            row![
                text("kerosene")
                    .size(18)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
                Space::new().width(Fill),
                text(ticker)
                    .size(24)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
            ]
            .align_y(Alignment::Center),
            Space::new().height(Fill),
            value_stack,
            Space::new().height(Fill),
            details,
        ]
        .spacing(10)
        .width(Fill)
        .height(Fill);

        let inner = container(card_content)
            .width(Fill)
            .height(Fill)
            .padding(18)
            .style(move |theme: &Theme| pnl_card_inner_style(theme, pnl_color));

        container(inner)
            .width(Fill)
            .height(300.0)
            .padding(4)
            .style(move |theme: &Theme| pnl_card_border_style(theme, pnl_color))
            .into()
    }
}

fn card_metric<'a>(
    label: &'static str,
    value: String,
    label_color: Color,
    value_color: Color,
) -> Element<'a, Message> {
    container(
        column![
            text(label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(label_color),
            text(value)
                .size(14)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
        ]
        .spacing(3),
    )
    .width(Fill)
    .into()
}
