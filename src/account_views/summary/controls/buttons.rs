use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, pick_list, row, text};
use iced::{Element, Length, Theme};

const DROPDOWN_CHEVRON_UP: &str = "\u{25B4}";
const DROPDOWN_CHEVRON_DOWN: &str = "\u{25BE}";

// ---------------------------------------------------------------------------
// Account Summary Window Buttons
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn summary_market_universe_picker(&self) -> Element<'_, Message> {
        let mut options = self.market_universe_options();
        if !options.contains(&self.market_universe) {
            options.push(self.market_universe.clone());
        }

        pick_list(
            options,
            Some(self.market_universe.clone()),
            Message::MarketUniverseChanged,
        )
        .padding([4, 8])
        .text_size(10)
        .width(Length::Shrink)
        .into()
    }

    pub(crate) fn summary_display_denomination_picker(&self) -> Element<'_, Message> {
        let mut options = self.display_denomination_options();
        if !options.contains(&self.display_denomination) {
            options.push(self.display_denomination.clone());
        }

        pick_list(
            options,
            Some(self.display_denomination.clone()),
            Message::DisplayDenominationChanged,
        )
        .padding([4, 8])
        .text_size(10)
        .width(Length::Shrink)
        .into()
    }

    pub(crate) fn summary_widgets_button(&self) -> Element<'_, Message> {
        let chevron = if self.add_widget_menu_open {
            DROPDOWN_CHEVRON_UP
        } else {
            DROPDOWN_CHEVRON_DOWN
        };
        button(
            row![
                text("Widgets").size(10).width(Length::Fixed(42.0)),
                text(chevron)
                    .size(13)
                    .width(Length::Fixed(10.0))
                    .align_x(iced::alignment::Horizontal::Center),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ToggleAddWidgetMenu)
        .padding([4, 8])
        .style(summary_primary_action_style)
        .into()
    }

    pub(crate) fn summary_layouts_button(&self) -> Element<'_, Message> {
        let chevron = if self.layout_menu_open {
            DROPDOWN_CHEVRON_UP
        } else {
            DROPDOWN_CHEVRON_DOWN
        };
        button(
            row![
                text(self.layout_switcher_button_label()).size(10),
                text(chevron)
                    .size(13)
                    .width(Length::Fixed(10.0))
                    .align_x(iced::alignment::Horizontal::Center),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ToggleLayoutMenu)
        .padding([4, 8])
        .style(summary_primary_action_style)
        .into()
    }

    pub(crate) fn summary_settings_button(&self) -> Element<'_, Message> {
        button(text("\u{2699}").size(12).center())
            .on_press(Message::OpenSettingsWindow)
            .padding([4, 8])
            .style(summary_primary_action_style)
            .into()
    }
}

fn summary_primary_action_style(theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ => theme.extended_palette().background.weak.color,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}
