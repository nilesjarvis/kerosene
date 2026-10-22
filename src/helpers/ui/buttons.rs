use crate::config::effective_radius;
use crate::helpers::design::{UiColors, filled_action_pair, mix_color, soft_action_pair};
use crate::message::Message;
use iced::widget::{button, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Shared Buttons
// ---------------------------------------------------------------------------

pub fn selected_row_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let bg = match status {
        button::Status::Hovered if !selected => theme.extended_palette().background.weak.color,
        _ => theme.extended_palette().background.base.color,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            width: 1.0,
            color: if selected {
                theme.palette().primary
            } else {
                Color::TRANSPARENT
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Mutually exclusive order-type segment (Market/Limit/Chase/TWAP). Labels
/// stay 12px in every state — selection changes fill and outline, never type
/// size (design language, audit D01). Callers place these on the shared inset
/// segment track.
pub fn order_type_button(
    label: &str,
    active: bool,
    msg: Message,
    pane_corner_radius: f32,
) -> Element<'static, Message> {
    button(text(label.to_owned()).size(12).center().width(Fill))
        .on_press(msg)
        .height(28)
        .padding([0, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            crate::helpers::design::compact_button_style(theme, status, active, pane_corner_radius)
        })
        .into()
}

/// Direction of a ticket trade action.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TradeSide {
    Buy,
    Sell,
}

impl TradeSide {
    fn base_color(self) -> fn(&Theme) -> Color {
        match self {
            TradeSide::Buy => |theme: &Theme| theme.palette().success,
            TradeSide::Sell => |theme: &Theme| theme.palette().danger,
        }
    }
}

/// State of a ticket trade action. Idle, busy, and unavailable share one
/// 36px frame with the same label position, so the action row never changes
/// size or family across the order lifecycle (design language, audit D05).
pub enum TradeAction {
    /// Submits the order.
    Place(Box<Message>),
    /// Submission in flight: the retained directional label with a reserved
    /// leading spinner slot, activation unavailable.
    Busy,
    /// Trading currently unavailable: quiet directional identity without
    /// activation.
    Unavailable,
}

/// One trade action component for the order ticket: 36px prominent control,
/// 13px label, filled direction color with a contrast-checked paired
/// foreground. Buy and sell have identical dimensions and treatment.
pub fn trade_action_button<'a>(
    side: TradeSide,
    label: String,
    action: TradeAction,
    spinner: Option<Element<'a, Message>>,
    pane_corner_radius: f32,
) -> Element<'a, Message> {
    let busy = matches!(action, TradeAction::Busy);
    let unavailable = matches!(action, TradeAction::Unavailable);

    let label_text = text(label)
        .size(13)
        .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(16.0)))
        .center();

    let content: iced::widget::Row<'a, Message> = match spinner {
        Some(spinner) if busy => iced::widget::row![spinner, label_text]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .width(Fill),
        _ => iced::widget::row![label_text].width(Fill),
    };

    let mut btn = button(content)
        .height(36)
        .padding([0, 12])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            trade_action_style(side, theme, status, unavailable, pane_corner_radius)
        });

    if let TradeAction::Place(msg) = action {
        btn = btn.on_press(*msg);
    }
    btn.into()
}

fn trade_action_style(
    side: TradeSide,
    theme: &Theme,
    status: button::Status,
    unavailable: bool,
    pane_corner_radius: f32,
) -> button::Style {
    let base_color = (side.base_color())(theme);
    if unavailable {
        // Quiet identity: soft directional fill, readable foreground, and no
        // pointer affordance while trading is unavailable.
        let (fill, foreground) = soft_action_pair(base_color, UiColors::new(theme).panel);
        return button::Style {
            background: Some(fill.into()),
            text_color: foreground,
            border: iced::Border {
                radius: effective_radius(pane_corner_radius, 4.0).into(),
                ..Default::default()
            },
            ..Default::default()
        };
    }

    let (fill, foreground) = filled_action_pair(base_color);
    let amount = match status {
        button::Status::Hovered => 0.06,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    let fill = mix_color(fill, foreground, amount);
    button::Style {
        background: Some(fill.into()),
        text_color: foreground,
        border: iced::Border {
            radius: effective_radius(pane_corner_radius, 4.0).into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
