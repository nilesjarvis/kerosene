use crate::account_state::AccountPickerOption;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{button, column, container, row, text, tooltip};
use iced::{border, Element, Fill, Length, Theme};

const ACCOUNT_PICKER_WIDTH: f32 = 250.0;
const ACCOUNT_PICKER_HEIGHT: f32 = 36.0;
const ACCOUNT_PICKER_BORDER_WIDTH: f32 = 1.0;
const ACCOUNT_PICKER_TRIGGER_WIDTH: f32 = 34.0;
const ACCOUNT_PICKER_RADIUS: f32 = 4.0;
const CHEVRON_UP: &str = "\u{25B4}";
const CHEVRON_DOWN: &str = "\u{25BE}";

#[derive(Debug, Clone, Copy)]
enum AccountPickerSegment {
    Label,
    Trigger,
}

impl TradingTerminal {
    pub(crate) fn view_account_picker_button(&self, theme: &Theme) -> Element<'_, Message> {
        let selected = self
            .account_picker_options()
            .into_iter()
            .find(|option| option.index == self.active_account_index)
            .unwrap_or(AccountPickerOption {
                index: self.active_account_index,
                label: "No account".to_string(),
                address: String::new(),
                can_trade: false,
                is_ghost: false,
            });

        let label = Self::truncate_display_text(&Self::account_picker_label(&selected), 20);
        let address = Self::account_picker_address_line(&selected);
        let chevron = if self.account_picker_open {
            CHEVRON_UP
        } else {
            CHEVRON_DOWN
        };
        let can_copy_address = !selected.address.trim().is_empty();
        let copy_message =
            can_copy_address.then(|| Message::CopyToClipboard(selected.address.clone()));
        let inner_width = ACCOUNT_PICKER_WIDTH - ACCOUNT_PICKER_BORDER_WIDTH * 2.0;
        let inner_height = ACCOUNT_PICKER_HEIGHT - ACCOUNT_PICKER_BORDER_WIDTH * 2.0;

        let label_button = button(
            row![
                column![
                    text(label).size(12).color(theme.palette().text),
                    text(address)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(1)
                .width(Fill),
                Self::account_mode_tag(selected.is_ghost, selected.can_trade, theme),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press_maybe(copy_message)
        .padding([4, 8])
        .width(Fill)
        .height(Length::Fixed(inner_height))
        .style(|theme: &Theme, status| {
            account_picker_segment_style(theme, status, AccountPickerSegment::Label, false)
        });

        let label_segment: Element<'_, Message> = if can_copy_address {
            tooltip(
                label_button,
                text("Copy address").size(10),
                tooltip::Position::Bottom,
            )
            .into()
        } else {
            label_button.into()
        };

        let trigger_active = self.account_picker_open;
        let trigger_segment = button(
            text(chevron)
                .size(13)
                .width(Fill)
                .center()
                .color(theme.extended_palette().background.weak.text),
        )
        .on_press(Message::ToggleAccountPicker)
        .padding([4, 0])
        .width(Length::Fixed(ACCOUNT_PICKER_TRIGGER_WIDTH))
        .height(Length::Fixed(inner_height))
        .style(move |theme: &Theme, status| {
            account_picker_segment_style(
                theme,
                status,
                AccountPickerSegment::Trigger,
                trigger_active,
            )
        });

        container(
            row![label_segment, trigger_segment]
                .spacing(0)
                .width(Length::Fixed(inner_width))
                .height(Length::Fixed(inner_height)),
        )
        .padding(ACCOUNT_PICKER_BORDER_WIDTH)
        .width(Length::Fixed(ACCOUNT_PICKER_WIDTH))
        .height(Length::Fixed(ACCOUNT_PICKER_HEIGHT))
        .style(move |theme: &Theme| account_picker_frame_style(theme, trigger_active))
        .into()
    }
}

fn account_picker_frame_style(theme: &Theme, active: bool) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.weak.text;
    border_color.a = 0.24;

    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: ACCOUNT_PICKER_RADIUS.into(),
            width: ACCOUNT_PICKER_BORDER_WIDTH,
            color: if active {
                theme.palette().primary
            } else {
                border_color
            },
        },
        ..Default::default()
    }
}

fn account_picker_segment_style(
    theme: &Theme,
    status: button::Status,
    segment: AccountPickerSegment,
    active: bool,
) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ if active => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.weak.color,
    };

    let border_color = if active {
        theme.palette().primary
    } else {
        theme.extended_palette().background.strong.color
    };

    button::Style {
        background: Some(bg.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: match segment {
                AccountPickerSegment::Label => border::left(ACCOUNT_PICKER_RADIUS),
                AccountPickerSegment::Trigger => border::right(ACCOUNT_PICKER_RADIUS),
            },
            width: 0.0,
            color: border_color,
        },
        ..Default::default()
    }
}
