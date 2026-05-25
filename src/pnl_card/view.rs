mod editor;
mod preview;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::metrics::PnlCardMetrics;
use super::model::PnlCardWindowState;
use editor::view_pnl_card_editor;

use iced::widget::container as container_style;
use iced::widget::{button, column, container, scrollable, text, tooltip};
use iced::{Color, Element, Fill, Length, Theme, window};

// ---------------------------------------------------------------------------
// PnL Card Views
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_pnl_card_window(&self, window_id: window::Id) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(state) = self.pnl_card_windows.get(&window_id) else {
            return missing_pnl_card_view(&theme, "PnL card not found");
        };

        let content = self
            .pnl_card_metrics_for_state(state)
            .map(|metrics| self.view_pnl_card_content(window_id, state, metrics, &theme))
            .unwrap_or_else(|message| missing_pnl_card_view(&theme, message));

        container(scrollable(content).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.palette().background.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            })
            .into()
    }

    fn view_pnl_card_content<'a>(
        &'a self,
        window_id: window::Id,
        state: &'a PnlCardWindowState,
        metrics: PnlCardMetrics,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let card = self.view_pnl_card_preview(state, metrics, theme);
        let editor = view_pnl_card_editor(window_id, state, theme);

        column![card, editor]
            .spacing(14)
            .padding(18)
            .width(Fill)
            .height(Length::Shrink)
            .into()
    }
}

pub(crate) fn pnl_card_icon_button(
    message: Option<Message>,
    tooltip_label: &'static str,
) -> Element<'static, Message> {
    let button = button(text("\u{25F0}").size(10).center())
        .on_press_maybe(message)
        .padding([1, 4])
        .style(|theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(background.into()),
                text_color: theme.palette().primary,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    tooltip(
        button,
        text(tooltip_label)
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

fn missing_pnl_card_view<'a>(theme: &Theme, message: impl Into<String>) -> Element<'a, Message> {
    container(
        column![
            text("kerosene")
                .size(18)
                .font(crate::app_fonts::monospace_font()),
            text(message.into())
                .size(12)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(10)
        .padding(18),
    )
    .width(Fill)
    .height(Fill)
    .into()
}
