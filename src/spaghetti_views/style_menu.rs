use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti::ComparisonColorMode;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use iced::widget::container as container_style;
use iced::widget::{Column, button, checkbox, container, radio, row, rule, stack, text};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Comparison Style Menu
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_spaghetti_style_menu(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'static, Message> {
        let labels_checked = inst.canvas.effective_show_labels();

        let labels = checkbox(labels_checked)
            .label("Labels")
            .on_toggle(move |_| Message::ToggleSpaghettiLabels(id))
            .size(10)
            .spacing(4)
            .text_size(10)
            .font(crate::app_fonts::monospace_font())
            .width(Length::Fill);

        let color_options = ComparisonColorMode::ALL.into_iter().fold(
            Column::new().spacing(3).width(Fill),
            |column, mode| {
                column.push(style_mode_option(
                    mode,
                    inst.canvas.color_mode,
                    move |selected| Message::SpaghettiSetColorMode(id, selected),
                ))
            },
        );

        let menu_col = Column::new()
            .spacing(5)
            .padding(6)
            .width(Fill)
            .push(style_group("LINE", labels.into()))
            .push(compact_separator())
            .push(style_group("COLOR", color_options.into()));

        let menu_card =
            container(menu_col)
                .width(220.0)
                .style(|theme: &Theme| container_style::Style {
                    background: Some(theme.extended_palette().background.strong.color.into()),
                    border: iced::Border {
                        color: theme.extended_palette().background.weak.color,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                });

        let bg_overlay = button("")
            .width(Fill)
            .height(Fill)
            .on_press(Message::CloseAllMenus)
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            });

        stack![
            bg_overlay,
            container(menu_card)
                .width(Fill)
                .height(Fill)
                .padding([30, 20])
                .align_x(Alignment::Start)
                .align_y(Alignment::Start)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}

// ---------------------------------------------------------------------------
// Comparison Style Components
// ---------------------------------------------------------------------------

fn style_group(
    label: &'static str,
    content: Element<'static, Message>,
) -> Element<'static, Message> {
    row![
        container(
            text(label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(Color::from_rgb8(0x88, 0x88, 0x88))
        )
        .width(40.0),
        content
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .width(Fill)
    .into()
}

fn style_mode_option(
    mode: ComparisonColorMode,
    selected: ComparisonColorMode,
    on_select: impl Fn(ComparisonColorMode) -> Message + 'static,
) -> Element<'static, Message> {
    radio(mode.to_string(), mode, Some(selected), on_select)
        .size(11)
        .spacing(5)
        .text_size(10)
        .width(Length::Fill)
        .into()
}

fn compact_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.16,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}
