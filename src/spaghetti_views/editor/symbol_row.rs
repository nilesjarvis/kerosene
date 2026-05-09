use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::helpers::category_color;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;
use iced::widget::{button, row, text};
use iced::{Element, Fill, Theme, color};

impl TradingTerminal {
    pub(super) fn view_spaghetti_editor_symbol_row<'a>(
        &'a self,
        id: SpaghettiChartId,
        sym: &'a ExchangeSymbol,
        is_added: bool,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let display = sym.display_name.as_deref().unwrap_or(&sym.ticker);
        let cat_badge = text(sym.category.to_uppercase())
            .size(9)
            .color(category_color(&sym.category, theme));

        let key = sym.key.clone();
        button(
            row![
                text(if is_added { "✓" } else { "" })
                    .size(10)
                    .color(theme.palette().success)
                    .width(16),
                text(display).size(12).width(Fill),
                cat_badge,
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .on_press(if is_added {
            Message::SpaghettiRemoveSymbol(id, key)
        } else {
            Message::SpaghettiAddSymbol(id, key)
        })
        .padding([3, 6])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = if is_added {
                color!(0x2a2a4a)
            } else {
                match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => theme.extended_palette().background.strong.color,
                }
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }
}
