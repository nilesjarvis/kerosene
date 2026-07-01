use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, button, column, container, row, rule, scrollable, text};
use iced::{Color, Element, Fill, Theme};

mod option_row;
mod sections;

impl TradingTerminal {
    pub(crate) fn view_account_picker_dropdown(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let options = self.account_picker_options();

        let mut menu = Column::new()
            .spacing(4)
            .width(Fill)
            .push(sections::dropdown_title(&theme))
            .push(rule::horizontal(1))
            .push(sections::section_label("Saved Profiles", &theme));

        let mut saved_count = 0usize;
        for option in options.iter().filter(|option| !option.is_ghost) {
            saved_count += 1;
            menu = menu.push(self.view_account_picker_option_row(option, &theme));
        }
        if saved_count == 0 {
            menu = menu.push(sections::empty_saved_profiles(&theme));
        }

        if self.schwab.connected_account_count() > 0 {
            menu = menu
                .push(rule::horizontal(1))
                .push(sections::section_label("Schwab", &theme));
            for account in &self.schwab.accounts {
                menu = menu.push(self.view_schwab_account_picker_row(
                    account.account_hash.clone(),
                    account.label(),
                    account.masked_account_number(),
                    &theme,
                ));
            }
            if self.schwab.accounts.is_empty() {
                for account in &self.schwab.linked_accounts {
                    let label = format!("Schwab {}", account.masked_account_number());
                    menu = menu.push(self.view_schwab_account_picker_row(
                        account.hash_value.clone(),
                        label,
                        account.masked_account_number(),
                        &theme,
                    ));
                }
            }
        }

        if options.iter().any(|option| option.is_ghost) {
            menu = menu
                .push(rule::horizontal(1))
                .push(sections::section_label("Ghost Sessions", &theme));
            for option in options.iter().filter(|option| option.is_ghost) {
                menu = menu.push(self.view_account_picker_option_row(option, &theme));
            }
        }

        menu = menu
            .push(rule::horizontal(1))
            .push(sections::schwab_connect_button())
            .push(sections::add_account_button());

        if self.connected_address.is_some() {
            menu = menu
                .push(rule::horizontal(1))
                .push(sections::disconnect_account_button());
        }

        container(scrollable(menu).height(iced::Length::Shrink))
            .padding(10)
            .width(380)
            .max_height(420.0)
            .style(account_picker_dropdown_style)
            .into()
    }

    fn view_schwab_account_picker_row(
        &self,
        hash: String,
        label: String,
        account_line: String,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let is_active = self.active_account_source
            == crate::account_state::ActiveAccountSource::Schwab
            && self.schwab.selected_account_hash.as_deref() == Some(hash.as_str());
        let active_marker = if is_active { ">" } else { "" };
        let label = Self::truncate_display_text(&label, 28);

        button(
            row![
                text(active_marker)
                    .size(11)
                    .color(theme.palette().primary)
                    .width(12),
                column![
                    text(label).size(12).color(theme.palette().text),
                    text(account_line)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(2)
                .width(Fill),
                Self::account_integration_tag("SCHWAB", theme),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::SchwabAccountPickerSelected(Some(hash).into()))
        .padding([7, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if is_active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    width: if is_active { 1.0 } else { 0.0 },
                    color: if is_active {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    },
                },
                ..Default::default()
            }
        })
        .into()
    }
}

fn account_picker_dropdown_style(theme: &Theme) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.weak.text;
    border_color.a = 0.22;

    container_style::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    }
}
