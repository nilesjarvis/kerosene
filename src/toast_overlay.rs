use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::notification_state::Toast;

use iced::widget::container as container_style;
use iced::widget::{Column, button, container, row, text};
use iced::{Alignment, Color, Element, Length, Padding, Theme};

// ---------------------------------------------------------------------------
// Toast overlay view
// ---------------------------------------------------------------------------

/// Fixed width of a toast card.
const TOAST_WIDTH: f32 = 320.0;
/// Horizontal travel of the slide animation, in pixels.
const SLIDE_DISTANCE: f32 = 26.0;
/// Maximum simultaneously visible toasts.
const VISIBLE_TOASTS: usize = 5;

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        a: (color.a * alpha).clamp(0.0, 1.0),
        ..color
    }
}

/// Cubic ease-out used for the entrance motion.
fn ease_out(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// Cubic ease-in used for the exit motion.
fn ease_in(t: f32) -> f32 {
    t * t * t
}

impl TradingTerminal {
    pub(crate) fn view_toast_overlay(&self, theme: &Theme) -> Option<Element<'_, Message>> {
        if self.toasts.is_empty() {
            return None;
        }

        let now = std::time::Instant::now();
        let position = self.toast_position;
        let animate = self.toast_animations_enabled;

        let mut toast_col = Column::new().spacing(8).width(Length::Fixed(TOAST_WIDTH));
        for toast in visible_toasts(&self.toasts) {
            toast_col = toast_col.push(toast_card(theme, toast, now, animate, position));
        }

        // Anchor the stack to the configured corner with matching edge padding.
        let align_x = if position.is_right() {
            Alignment::End
        } else {
            Alignment::Start
        };
        let align_y = if position.is_bottom() {
            Alignment::End
        } else {
            Alignment::Start
        };

        let padding = Padding {
            top: if position.is_bottom() { 0.0 } else { 10.0 },
            right: if position.is_right() { 10.0 } else { 0.0 },
            bottom: if position.is_bottom() { 10.0 } else { 0.0 },
            left: if position.is_right() { 0.0 } else { 10.0 },
        };

        Some(
            container(toast_col)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(padding)
                .align_x(align_x)
                .align_y(align_y)
                .into(),
        )
    }
}

fn visible_toasts(toasts: &[Toast]) -> Vec<&Toast> {
    let mut visible = Vec::with_capacity(VISIBLE_TOASTS);

    visible.extend(
        toasts
            .iter()
            .rev()
            .filter(|toast| toast.is_error)
            .take(VISIBLE_TOASTS),
    );

    if visible.len() < VISIBLE_TOASTS {
        visible.extend(
            toasts
                .iter()
                .rev()
                .filter(|toast| !toast.is_error)
                .take(VISIBLE_TOASTS - visible.len()),
        );
    }

    visible
}

fn toast_card<'a>(
    theme: &Theme,
    toast: &'a Toast,
    now: std::time::Instant,
    animate: bool,
    position: crate::config::ToastPosition,
) -> Element<'a, Message> {
    // Combine entrance and exit animation into a single 0.0..=1.0 visibility
    // value, plus a horizontal slide offset.
    let (visibility, slide) = if animate {
        let enter = ease_out(toast.enter_progress(now));
        let exit = ease_in(toast.exit_progress(now));
        let visibility = (enter - exit).clamp(0.0, 1.0);
        // Slide in from (and out toward) the anchored edge.
        let direction = if position.is_right() { 1.0 } else { -1.0 };
        let offset = (1.0 - enter) * SLIDE_DISTANCE + exit * SLIDE_DISTANCE;
        (visibility, direction * offset)
    } else {
        (1.0, 0.0)
    };

    let palette = theme.palette();
    let extended = theme.extended_palette();
    let accent = if toast.is_error {
        palette.danger
    } else {
        palette.success
    };
    let message_color = if toast.is_error {
        palette.danger
    } else {
        palette.text
    };

    let glyph = if toast.is_error { "!" } else { "✓" };
    let icon = container(
        text(glyph)
            .size(12)
            .color(with_alpha(Color::WHITE, visibility)),
    )
    .width(Length::Fixed(20.0))
    .height(Length::Fixed(20.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(move |_theme: &Theme| container_style::Style {
        background: Some(with_alpha(accent, 0.85 * visibility).into()),
        border: iced::Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let tid = toast.id;
    let dismiss_color = with_alpha(extended.background.weak.text, visibility);
    let dismiss = button(text("×").size(14))
        .on_press(Message::DismissToast(tid))
        .padding([0, 6])
        .style(move |_theme: &Theme, status| {
            let color = match status {
                button::Status::Hovered => with_alpha(palette.text, visibility),
                _ => dismiss_color,
            };
            button::Style {
                background: None,
                text_color: color,
                ..Default::default()
            }
        });

    let body = row![
        icon,
        text(&toast.message)
            .size(12)
            .color(with_alpha(message_color, visibility))
            .width(Length::Fill),
        dismiss,
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let card_bg = with_alpha(extended.background.strong.color, 0.97 * visibility);
    let border_color = with_alpha(accent, 0.55 * visibility);
    let shadow_color = with_alpha(Color::BLACK, 0.30 * visibility);

    let card = container(body)
        .padding(Padding {
            top: 9.0,
            right: 10.0,
            bottom: 9.0,
            left: 10.0,
        })
        .width(Length::Fill)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(card_bg.into()),
            border: iced::Border {
                radius: 8.0.into(),
                width: 1.0,
                color: border_color,
            },
            shadow: iced::Shadow {
                color: shadow_color,
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        });

    // Apply the horizontal slide by padding the leading/trailing edge.
    let slide_padding = if slide >= 0.0 {
        Padding {
            left: slide,
            ..Padding::ZERO
        }
    } else {
        Padding {
            right: -slide,
            ..Padding::ZERO
        }
    };

    container(card)
        .width(Length::Fill)
        .padding(slide_padding)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toast(id: u64, is_error: bool) -> Toast {
        Toast {
            id,
            message: format!("toast {id}"),
            is_error,
            created_at: std::time::Instant::now(),
            dismissing_at: None,
        }
    }

    #[test]
    fn visible_toasts_prioritize_errors_over_newer_info() {
        let mut toasts = vec![toast(0, true)];
        for id in 1..=VISIBLE_TOASTS as u64 {
            toasts.push(toast(id, false));
        }

        let visible = visible_toasts(&toasts);

        assert_eq!(visible.len(), VISIBLE_TOASTS);
        assert!(visible.iter().any(|toast| toast.id == 0 && toast.is_error));
        assert!(
            !visible.iter().any(|toast| toast.id == 1 && !toast.is_error),
            "oldest info toast should yield the visible slot to the error"
        );
    }

    #[test]
    fn visible_toasts_keep_latest_errors_first() {
        let toasts = vec![
            toast(0, true),
            toast(1, false),
            toast(2, true),
            toast(3, false),
        ];

        let visible = visible_toasts(&toasts);

        assert_eq!(
            visible.iter().map(|toast| toast.id).collect::<Vec<_>>(),
            vec![2, 0, 3, 1]
        );
    }
}
