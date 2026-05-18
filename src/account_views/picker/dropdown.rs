use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, container, rule, scrollable};
use iced::{Element, Fill, Theme};

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
            .push(sections::add_account_button());

        container(scrollable(menu).height(iced::Length::Shrink))
            .padding(10)
            .width(380)
            .max_height(420.0)
            .style(account_picker_dropdown_style)
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
