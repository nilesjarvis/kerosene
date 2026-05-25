use crate::message::Message;

use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{button, rule, svg, text};
use iced::{Color, Element, Length, Theme};

// ---------------------------------------------------------------------------
// Screenshot View Components
// ---------------------------------------------------------------------------

pub(super) const CAMERA_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M14.5 4l1.6 2H20a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4
           a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h3.9l1.6-2h5z"/>
  <circle cx="12" cy="13" r="4"/>
</svg>
"#;

pub(super) const CHEVRON_DOWN_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="m6 9 6 6 6-6"/>
</svg>
"#;

pub(super) fn chart_screenshot_button(
    label: &'static str,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label).size(12).center())
        .on_press(msg)
        .padding([6, 12])
        .style(|theme: &Theme, status| {
            let ext = theme.extended_palette();
            let bg = match status {
                button::Status::Hovered => ext.background.strong.color,
                _ => ext.background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn chart_screenshot_svg_icon(
    svg_bytes: &'static [u8],
    size: f32,
) -> Element<'static, Message> {
    svg(SvgHandle::from_memory(svg_bytes))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text),
        })
        .into()
}

pub(super) fn chart_screenshot_icon_button(
    icon: Element<'static, Message>,
    msg: Message,
    active: bool,
    padding: [u16; 2],
) -> Element<'static, Message> {
    button(icon)
        .on_press(msg)
        .padding(padding)
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if active => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn screenshot_menu_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.16,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}
