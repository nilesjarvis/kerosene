use crate::config::effective_radius;
use crate::helpers::design::{UiColors, compact_button_style};
use crate::message::Message;
use crate::positioning_state::{PositioningInfoId, PositioningInfoPage};

use iced::widget::{button, container, row, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Positioning Control Buttons
// ---------------------------------------------------------------------------

/// Content-stack (filter/control) choice button. Same compact recipe as the
/// book's tick and mode segments so pane-local choice controls stay one
/// family; pass `pane_corner_radius` from the owning pane.
pub(in crate::market_views::positioning_info) fn positioning_control_button(
    label: &'static str,
    active: bool,
    msg: Message,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    button(text(label).size(12).center())
        .on_press(msg)
        .height(24)
        .padding([0, 8])
        .style(move |theme: &Theme, status| {
            compact_button_style(theme, status, active, pane_corner_radius)
        })
        .into()
}

/// Compact constant-geometry filter reset; unavailable (kept inert, not
/// hidden) when there is nothing to clear.
pub(in crate::market_views::positioning_info) fn positioning_clear_filters_button(
    id: PositioningInfoId,
    active: bool,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    let mut clear_button = button(text("Clear filters").size(12).center())
        .height(24)
        .padding([0, 8])
        .style(move |theme: &Theme, status| {
            compact_button_style(theme, status, false, pane_corner_radius)
        });
    if active {
        clear_button = clear_button.on_press(Message::ClearPositioningInfoFilters(id));
    }
    clear_button.into()
}

/// Content navigation between the pane's pages (Positions/Change): active
/// page carries the shared selected-toggle treatment; inactive pages stay
/// readable muted labels so mode choice and content navigation remain two
/// distinct families.
pub(in crate::market_views::positioning_info) fn positioning_navigation_button(
    id: PositioningInfoId,
    page: PositioningInfoPage,
    active: bool,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    button(
        text(page.label())
            .size(12)
            .center()
            .style(move |theme: &Theme| text::Style {
                color: Some(if active {
                    UiColors::new(theme).accent
                } else {
                    UiColors::new(theme).muted
                }),
            }),
    )
    .on_press(Message::PositioningInfoPageChanged(id, page))
    .height(24)
    .padding([0, 10])
    .style(move |theme: &Theme, status| {
        compact_button_style(theme, status, active, pane_corner_radius)
    })
    .into()
}

/// Shared inset track behind a segmented group (side filters, timeframe
/// choices) matching the book's mode segments.
pub(in crate::market_views::positioning_info) fn positioning_segment_track(
    segments: iced::widget::Row<'static, Message>,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    container(segments)
        .padding(2)
        .style(move |theme: &Theme| container::Style {
            background: Some(UiColors::new(theme).inset.into()),
            border: iced::Border {
                radius: effective_radius(pane_corner_radius, 4.0).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// Right-aligned filler that keeps trailing controls on the content grid.
pub(in crate::market_views::positioning_info) fn positioning_filler() -> Element<'static, Message> {
    row![].width(Fill).into()
}
