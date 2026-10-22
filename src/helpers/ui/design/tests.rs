use super::*;
use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;

#[test]
fn resolved_controls_are_readable_across_built_in_and_custom_themes() {
    let (terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig::default());
    let names: Vec<_> = Theme::ALL
        .iter()
        .map(ToString::to_string)
        .chain(
            terminal
                .custom_themes
                .iter()
                .map(|t| format!("Custom: {}", t.name)),
        )
        .collect();
    for name in names {
        let theme = terminal.get_theme_by_name(&name);
        let colors = UiColors::new(&theme);
        for color in [
            colors.text,
            colors.muted,
            colors.accent,
            colors.positive,
            colors.negative,
            colors.warning,
        ] {
            assert!(contrast_ratio(color, colors.panel) >= 4.5, "{name}");
        }
        for selected in [false, true] {
            for status in [
                button::Status::Active,
                button::Status::Hovered,
                button::Status::Pressed,
                button::Status::Disabled,
            ] {
                let style = compact_button_style(&theme, status, selected, 6.0);
                let background = match style.background {
                    Some(iced::Background::Color(color)) => color,
                    _ => colors.panel,
                };
                assert!(
                    contrast_ratio(style.text_color, background) >= 4.5,
                    "{name} {status:?}"
                );
                if selected {
                    assert!(
                        contrast_ratio(style.border.color, background) >= 3.0,
                        "{name}"
                    );
                }
            }
        }
        for invalid in [false, true] {
            for status in [
                text_input::Status::Active,
                text_input::Status::Hovered,
                text_input::Status::Focused { is_hovered: false },
                text_input::Status::Focused { is_hovered: true },
                text_input::Status::Disabled,
            ] {
                let field = compact_input_style(&theme, status, invalid, 6.0);
                let iced::Background::Color(background) = field.background else {
                    panic!("solid field surface");
                };
                assert!(contrast_ratio(field.value, background) >= 4.5, "{name}");
                assert!(
                    contrast_ratio(field.value, field.selection) >= 4.5,
                    "{name}"
                );
                assert!(
                    contrast_ratio(field.placeholder, background) >= 4.5,
                    "{name}"
                );
                assert!(
                    contrast_ratio(field.border.color, background) >= 3.0,
                    "{name}"
                );
                assert_eq!(
                    field.border.width,
                    if matches!(status, text_input::Status::Focused { .. }) {
                        2.0
                    } else {
                        1.0
                    }
                );
            }
        }
    }
}

#[test]
fn selected_controls_keep_their_marker_and_geometry_when_hovered_or_pressed() {
    let theme = Theme::Light;
    let active = compact_button_style(&theme, button::Status::Active, true, 6.0);
    for status in [button::Status::Hovered, button::Status::Pressed] {
        let style = compact_button_style(&theme, status, true, 6.0);
        assert_eq!(style.border.width, active.border.width);
        assert_eq!(style.border.radius, active.border.radius);
        assert_eq!(style.border.color.a, 1.0);
        assert_ne!(style.background, active.background);
    }
    assert_eq!(
        compact_button_style(&theme, button::Status::Active, true, 0.0)
            .border
            .radius,
        0.0.into()
    );
}

#[test]
fn derived_roles_preserve_surface_opacity_and_correct_low_contrast_custom_colors() {
    let surface = Color::from_rgba(0.5, 0.5, 0.5, 0.4);
    assert_eq!(mix_color(surface, Color::WHITE, 0.12).a, 0.4);
    for background in [Color::BLACK, Color::WHITE, surface] {
        let readable = readable_color(background, background, 4.5);
        assert!(contrast_ratio(readable, background) >= 4.5);
        assert_eq!(readable.a, 1.0);
    }
}
