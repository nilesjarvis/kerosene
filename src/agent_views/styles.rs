use crate::helpers;
use iced::widget::container as container_style;
use iced::widget::{button, text_input};
use iced::{Border, Color, Theme};

pub(super) fn with_alpha(mut color: Color, alpha: f32) -> Color {
    color.a *= alpha.clamp(0.0, 1.0);
    color
}

pub(super) fn agent_response_action_style(
    theme: &Theme,
    status: button::Status,
    progress: f32,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = theme.palette().text;
    background.a = if hovered { 0.07 * progress } else { 0.0 };
    button::Style {
        background: Some(background.into()),
        text_color: with_alpha(theme.palette().text, progress),
        border: Border {
            radius: 5.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_follow_up_style(
    theme: &Theme,
    status: button::Status,
    progress: f32,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = theme.palette().primary;
    background.a = if hovered { 0.07 * progress } else { 0.0 };
    let mut border = theme.extended_palette().background.strong.color;
    border.a *= progress;
    button::Style {
        background: Some(background.into()),
        text_color: with_alpha(theme.palette().text, progress),
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: border,
        },
        ..Default::default()
    }
}

pub(super) fn chip_style(theme: &Theme, color: Color) -> container_style::Style {
    let mut background = color;
    background.a = 0.08;
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            radius: 99.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

pub(super) fn user_bubble_style(theme: &Theme) -> container_style::Style {
    let mut background = theme.palette().primary;
    background.a = 0.12;
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.palette().primary,
        },
        ..Default::default()
    }
}

pub(super) fn agent_reasoning_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut background = theme.palette().text;
    background.a = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        0.045
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.extended_palette().background.weak.text,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_reasoning_rail_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        ..Default::default()
    }
}

pub(super) fn agent_tool_trace_row_style(
    _theme: &Theme,
    state_color: Option<Color>,
) -> container_style::Style {
    container_style::Style {
        background: state_color.map(|mut color| {
            color.a = 0.045;
            color.into()
        }),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_session_sidebar_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..Default::default()
    }
}

pub(super) fn agent_sidebar_control_button_style(
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let mut background = theme.palette().text;
    background.a = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        0.06
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: if matches!(status, button::Status::Disabled) {
            theme.extended_palette().background.weak.text
        } else {
            with_alpha(theme.palette().text, 0.82)
        },
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_session_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = theme.palette().text;
    background.a = if active {
        0.075
    } else if hovered {
        0.05
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_prompt_model_button_style(
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let mut background = theme.palette().primary;
    background.a = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        0.09
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_composer_input_style(
    theme: &Theme,
    status: text_input::Status,
) -> text_input::Style {
    let mut style = helpers::text_input_style(theme, status);
    style.background = Color::TRANSPARENT.into();
    style.border = Border::default();
    style
}

pub(super) fn agent_composer_style(theme: &Theme) -> container_style::Style {
    let mut shadow_color = Color::BLACK;
    shadow_color.a = 0.16;
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        shadow: iced::Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

pub(super) fn agent_composer_add_button_style(
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let disabled = matches!(status, button::Status::Disabled);
    let mut background = theme.palette().text;
    background.a = if disabled {
        0.0
    } else if hovered {
        0.08
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: if disabled {
            theme.extended_palette().background.weak.text
        } else {
            theme.palette().text
        },
        border: Border {
            radius: 7.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_prompt_action_button_style(
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    let pressed = matches!(status, button::Status::Pressed);
    let mut background = if disabled {
        theme.extended_palette().background.strong.color
    } else {
        theme.palette().primary
    };
    if pressed {
        background.a *= 0.82;
    }
    button::Style {
        background: Some(background.into()),
        text_color: if disabled {
            theme.extended_palette().background.weak.text
        } else {
            theme.extended_palette().primary.base.text
        },
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn agent_model_picker_style(theme: &Theme) -> container_style::Style {
    let mut shadow_color = Color::BLACK;
    shadow_color.a = 0.20;
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        shadow: iced::Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

pub(super) fn agent_model_option_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = if selected {
        theme.palette().primary
    } else {
        theme.palette().text
    };
    background.a = if selected {
        0.12
    } else if hovered {
        0.055
    } else {
        0.0
    };
    let mut border_color = theme.palette().primary;
    border_color.a = if selected { 0.55 } else { 0.0 };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    }
}

pub(super) fn agent_empty_card_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

pub(super) fn agent_pnl_card_hover_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(with_alpha(theme.palette().primary, 0.10).into()),
        border: Border {
            color: with_alpha(theme.palette().primary, 0.8),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..Default::default()
    }
}
