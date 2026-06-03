use crate::app_state::TradingTerminal;
use crate::config::ToastPosition;
use crate::message::Message;

use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke};
use iced::widget::{Column, Row, button, checkbox, column, text};
use iced::{Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme};

// ---------------------------------------------------------------------------
// Notification (toast) appearance settings
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_settings_notifications_section(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let weak_text = theme.extended_palette().background.weak.text;

        column![
            text("Toast position").size(13).color(theme.palette().text),
            text("Choose which corner notifications appear in.")
                .size(11)
                .color(weak_text),
            position_grid(&theme, self.toast_position),
            checkbox(self.toast_animations_enabled)
                .label("Slide and fade animation")
                .on_toggle(Message::ToggleToastAnimations)
                .size(12)
                .spacing(8)
                .text_size(12)
                .font(crate::app_fonts::monospace_font()),
            text("Animations ease toasts in and out; disable for instant pop-in.")
                .size(11)
                .color(weak_text),
        ]
        .spacing(10)
        .into()
    }
}

fn position_grid(theme: &Theme, selected: ToastPosition) -> Element<'static, Message> {
    let mut grid = Column::new().spacing(8).width(Fill);

    for positions in ToastPosition::ALL.chunks(2) {
        let mut row = Row::new().spacing(8).width(Fill);
        for position in positions {
            row = row.push(position_card(theme, *position, selected));
        }
        grid = grid.push(row);
    }

    grid.into()
}

fn position_card(
    theme: &Theme,
    position: ToastPosition,
    selected: ToastPosition,
) -> Element<'static, Message> {
    let is_selected = position == selected;
    let label_color = if is_selected {
        theme.palette().primary
    } else {
        theme.extended_palette().background.weak.text
    };

    let preview: Element<'static, Message> = iced::widget::canvas(PositionPreview { position })
        .width(Fill)
        .height(Length::Fixed(56.0))
        .into();

    let content = column![
        preview,
        text(position.label())
            .size(10)
            .color(label_color)
            .font(crate::app_fonts::monospace_font()),
    ]
    .spacing(6)
    .align_x(Alignment::Center)
    .width(Fill);

    button(content)
        .on_press(Message::ToastPositionChanged(position))
        .padding([8, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let extended = theme.extended_palette();
            let background = match status {
                button::Status::Hovered => extended.background.strong.color,
                _ if is_selected => Color {
                    a: 0.38,
                    ..extended.background.strong.color
                },
                _ => Color {
                    a: 0.22,
                    ..extended.background.weak.color
                },
            };
            let border_color = if is_selected {
                theme.palette().primary
            } else {
                Color {
                    a: 0.28,
                    ..extended.background.weak.text
                }
            };

            button::Style {
                background: Some(background.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    color: border_color,
                    width: if is_selected { 1.0 } else { 0.5 },
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

struct PositionPreview {
    position: ToastPosition,
}

impl Program<Message> for PositionPreview {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let extended = theme.extended_palette();

        // Mini window outline.
        let window_inset = 4.0;
        let window = Rectangle {
            x: window_inset,
            y: window_inset,
            width: bounds.width - window_inset * 2.0,
            height: bounds.height - window_inset * 2.0,
        };
        let window_path = Path::rectangle(
            Point::new(window.x, window.y),
            Size::new(window.width, window.height),
        );
        frame.fill(
            &window_path,
            Color {
                a: 0.35,
                ..extended.background.weak.color
            },
        );
        frame.stroke(
            &window_path,
            Stroke::default()
                .with_color(Color {
                    a: 0.45,
                    ..extended.background.weak.text
                })
                .with_width(1.0),
        );

        // Toast chip placed in the configured corner.
        let chip_w = window.width * 0.46;
        let chip_h = 9.0;
        let pad = 5.0;
        let chip_x = if self.position.is_right() {
            window.x + window.width - pad - chip_w
        } else {
            window.x + pad
        };
        let chip_y = if self.position.is_bottom() {
            window.y + window.height - pad - chip_h
        } else {
            window.y + pad
        };
        let chip = Path::rectangle(Point::new(chip_x, chip_y), Size::new(chip_w, chip_h));
        frame.fill(&chip, theme.palette().primary);

        // Accent strip on the leading edge of the chip.
        let strip = Path::rectangle(Point::new(chip_x, chip_y), Size::new(2.5, chip_h));
        frame.fill(&strip, theme.palette().success);

        vec![frame.into_geometry()]
    }
}
