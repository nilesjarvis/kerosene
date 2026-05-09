use iced::widget::text_input;
use iced::{Background, Border, Color, Theme};

// ---------------------------------------------------------------------------
// Input Styles
// ---------------------------------------------------------------------------

pub fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let ext = theme.extended_palette();
    match status {
        text_input::Status::Active => text_input::Style {
            background: Background::Color(Color {
                a: 0.05,
                ..palette.text
            }),
            border: Border {
                color: Color {
                    a: 0.1,
                    ..palette.text
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            icon: ext.background.weak.text,
            placeholder: ext.background.weak.text,
            value: palette.text,
            selection: ext.primary.weak.color,
        },
        text_input::Status::Hovered => text_input::Style {
            background: Background::Color(ext.background.strong.color),
            border: Border {
                color: ext.background.weak.text,
                width: 1.0,
                radius: 4.0.into(),
            },
            icon: ext.background.weak.text,
            placeholder: ext.background.weak.text,
            value: palette.text,
            selection: ext.primary.weak.color,
        },
        text_input::Status::Focused { .. } => text_input::Style {
            background: Background::Color(ext.background.strong.color),
            border: Border {
                color: palette.primary,
                width: 1.0,
                radius: 4.0.into(),
            },
            icon: palette.text,
            placeholder: ext.background.weak.text,
            value: palette.text,
            selection: ext.primary.weak.color,
        },
        text_input::Status::Disabled => text_input::Style {
            background: Background::Color(ext.background.weak.color),
            border: Border {
                color: ext.background.weak.color,
                width: 1.0,
                radius: 4.0.into(),
            },
            icon: ext.background.weak.text,
            placeholder: ext.background.weak.text,
            value: ext.background.weak.text,
            selection: ext.primary.weak.color,
        },
    }
}
