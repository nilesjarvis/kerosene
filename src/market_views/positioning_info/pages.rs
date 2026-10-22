use super::{POSITIONING_PAGE_PADDING, POSITIONING_SCROLLBAR_SPACING, POSITIONING_SCROLLBAR_WIDTH};
use crate::app_state::TradingTerminal;
use crate::helpers::design::UiColors;
use crate::message::Message;
use crate::positioning_state::PositioningInfoInstance;

use iced::widget::{column, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill, Theme};

mod tables;

// ---------------------------------------------------------------------------
// Positioning Information Pages
// ---------------------------------------------------------------------------

/// Inline error notice: partial data keeps a warning tone, a failed load a
/// critical tone (resolved against the pane surface).
fn view_positioning_error_notice(error: &str, has_data: bool) -> Element<'static, Message> {
    let error = error.to_string();
    text(error)
        .size(11)
        .style(move |theme: &Theme| {
            let colors = UiColors::new(theme);
            text::Style {
                color: Some(if has_data {
                    colors.warning
                } else {
                    colors.negative
                }),
            }
        })
        .into()
}

impl TradingTerminal {
    pub(super) fn view_positioning_info_positions_page<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        now_ms: u64,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let controls = self.view_positioning_info_controls(instance);

        let mut content =
            column![self.view_positioning_info_title(instance, now_ms, theme, true)].spacing(8);
        if instance.symbol_picker_open {
            content = content.push(self.view_positioning_info_symbol_dropdown(instance, theme));
        }
        content = content.push(controls);

        if let Some(error) = &instance.error {
            content = content.push(view_positioning_error_notice(
                error.as_str(),
                instance.data.is_some(),
            ));
        }

        if let Some(data) = &instance.data {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_summary(data, theme))
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_table(
                    data,
                    instance,
                    now_ms,
                    available_width,
                    theme,
                ));
        } else {
            let status: Element<'_, Message> = if instance.loading {
                row![
                    self.view_spinner(18),
                    text("Loading positioning data...")
                        .size(12)
                        .style(move |theme: &Theme| text::Style {
                            color: Some(UiColors::new(theme).muted),
                        }),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else if instance.error.is_none() {
                text("No positioning data loaded")
                    .size(12)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).muted),
                    })
                    .into()
            } else {
                text("Positioning data unavailable")
                    .size(12)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).muted),
                    })
                    .into()
            };
            content = content
                .push(rule::horizontal(1))
                .push(container(status).width(Fill).height(Fill).center(Fill));
        }

        view_positioning_page_scroll(content)
    }

    pub(super) fn view_positioning_info_change_page<'a>(
        &'a self,
        instance: &'a PositioningInfoInstance,
        now_ms: u64,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let controls = self.view_positioning_info_change_controls(instance);

        let mut content =
            column![self.view_positioning_info_title(instance, now_ms, theme, true)].spacing(8);
        if instance.symbol_picker_open {
            content = content.push(self.view_positioning_info_symbol_dropdown(instance, theme));
        }
        content = content.push(controls);

        if let Some(error) = &instance.change_error {
            content = content.push(view_positioning_error_notice(
                error.as_str(),
                instance.change_data.is_some(),
            ));
        }

        if let Some(data) = &instance.change_data {
            content = content
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_change_summary(data, instance, theme))
                .push(rule::horizontal(1))
                .push(self.view_positioning_info_flow(instance, now_ms));
        } else {
            let status: Element<'_, Message> = if instance.change_loading {
                row![
                    self.view_spinner(18),
                    text("Loading position changes...")
                        .size(12)
                        .style(move |theme: &Theme| text::Style {
                            color: Some(UiColors::new(theme).muted),
                        }),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else if instance.change_error.is_none() {
                text("No change data loaded")
                    .size(12)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).muted),
                    })
                    .into()
            } else {
                text("Change data unavailable")
                    .size(12)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(UiColors::new(theme).muted),
                    })
                    .into()
            };
            content = content
                .push(rule::horizontal(1))
                .push(container(status).width(Fill).height(Fill).center(Fill));
        }

        view_positioning_page_scroll(content)
    }
}

fn view_positioning_page_scroll<'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        scrollable(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(POSITIONING_SCROLLBAR_WIDTH)
                    .scroller_width(POSITIONING_SCROLLBAR_WIDTH),
            ))
            .spacing(POSITIONING_SCROLLBAR_SPACING)
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .padding(POSITIONING_PAGE_PADDING)
    .into()
}
