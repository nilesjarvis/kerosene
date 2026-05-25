use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::positioning_state::PositioningInfoInstance;

use iced::widget::{column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Element, Fill, Theme};

mod tables;

// ---------------------------------------------------------------------------
// Positioning Information Pages
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_positioning_info_positions_page<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let search = text_input("Select perp ticker...", &instance.search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::PositioningInfoSearchChanged(instance.id, q))
            .size(12)
            .padding([5, 8]);
        let autocomplete =
            self.view_positioning_info_autocomplete(instance.id, &instance.search_query, theme);
        let controls = self.view_positioning_info_controls(instance);

        let mut content = column![
            self.view_positioning_info_title(instance, theme, false),
            search,
            autocomplete,
            controls,
        ]
        .spacing(8);

        if let Some(error) = &instance.error {
            content = content.push(text(error.clone()).size(11).color(
                if instance.data.is_some() {
                    theme.palette().warning
                } else {
                    theme.palette().danger
                },
            ));
        }

        if let Some(data) = &instance.data {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_summary(data, instance, theme))
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_table(data, instance, available_width, theme));
        } else {
            let status: Element<'_, Message> = if instance.loading {
                row![
                    self.view_spinner(18),
                    text("Loading positioning data...")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else if instance.error.is_none() {
                text("No positioning data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            } else {
                text("Positioning data unavailable")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            };
            content = content
                .push(rule::horizontal(1))
                .push(container(status).width(Fill).height(Fill).center(Fill));
        }

        container(scrollable(content))
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    pub(super) fn view_positioning_info_change_page<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let controls = self.view_positioning_info_change_controls(instance);

        let mut content =
            column![self.view_positioning_info_title(instance, theme, true)].spacing(8);
        if instance.symbol_picker_open {
            content = content.push(self.view_positioning_info_symbol_dropdown(instance, theme));
        }
        content = content.push(controls);

        if let Some(error) = &instance.change_error {
            content = content.push(text(error.clone()).size(11).color(
                if instance.change_data.is_some() {
                    theme.palette().warning
                } else {
                    theme.palette().danger
                },
            ));
        }

        if let Some(data) = &instance.change_data {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_change_summary(data, instance, theme))
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_change_table(
                    data,
                    instance,
                    available_width,
                    theme,
                ));
        } else {
            let status: Element<'_, Message> = if instance.change_loading {
                row![
                    self.view_spinner(18),
                    text("Loading position changes...")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else if instance.change_error.is_none() {
                text("No change data loaded")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            } else {
                text("Change data unavailable")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            };
            content = content
                .push(rule::horizontal(1))
                .push(container(status).width(Fill).height(Fill).center(Fill));
        }

        container(scrollable(content))
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }
}
