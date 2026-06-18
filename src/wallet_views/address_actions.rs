use crate::message::Message;

use iced::widget::svg::Handle as SvgHandle;
use iced::widget::text::Wrapping;
use iced::widget::{Row, button, container, mouse_area, rule, svg, text, tooltip};
use iced::{Color, Element, Fill, Length, Theme, mouse};

const WALLET_ACTION_CELL_HEIGHT: f32 = 20.0;
const WALLET_ACTION_ICON_SIZE: f32 = 13.0;
const WALLET_ACTION_SEPARATOR_HEIGHT: f32 = 12.0;

const COPY_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <rect width="14" height="14" x="8" y="8" rx="2" ry="2"/>
  <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>
</svg>
"#;

const DETACH_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M15 3h6v6"/>
  <path d="M10 14 21 3"/>
  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
</svg>
"#;

const GHOST_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M9 10h.01"/>
  <path d="M15 10h.01"/>
  <path d="M12 2a8 8 0 0 0-8 8v12l3-3 2.5 2.5L12 19l2.5 2.5L17 19l3 3V10a8 8 0 0 0-8-8z"/>
</svg>
"#;

pub(crate) struct WalletAddressActionCell<'a> {
    pub(crate) address: String,
    pub(crate) label: String,
    pub(crate) tooltip_label: String,
    pub(crate) hover_key: String,
    pub(crate) hovered_key: Option<&'a str>,
    pub(crate) width: f32,
    pub(crate) text_size: u16,
    pub(crate) text_color: Color,
}

pub(crate) fn wallet_address_action_cell(
    config: WalletAddressActionCell<'_>,
) -> Element<'static, Message> {
    let hovered = config.hovered_key == Some(config.hover_key.as_str());
    let hover_key = config.hover_key;
    let width = config.width.max(0.0);

    let content = if hovered {
        wallet_address_segments(config.address)
    } else {
        wallet_address_label(
            config.address,
            config.label,
            config.tooltip_label,
            config.text_size,
            config.text_color,
        )
    };

    mouse_area(
        container(content)
            .width(Length::Fixed(width))
            .height(Length::Fixed(WALLET_ACTION_CELL_HEIGHT)),
    )
    .on_enter(Message::WalletAddressActionsHovered(
        hover_key.clone().into(),
    ))
    .on_exit(Message::WalletAddressActionsExited(hover_key.into()))
    .interaction(mouse::Interaction::Pointer)
    .into()
}

fn wallet_address_label(
    address: String,
    label: String,
    tooltip_label: String,
    text_size: u16,
    text_color: Color,
) -> Element<'static, Message> {
    let content = text(label)
        .size(u32::from(text_size))
        .font(crate::app_fonts::monospace_font())
        .color(text_color)
        .wrapping(Wrapping::None)
        .width(Fill);

    tooltip(
        button(content)
            .on_press(Message::CopyToClipboard(address))
            .padding([0, 4])
            .width(Fill)
            .height(Length::Fixed(WALLET_ACTION_CELL_HEIGHT))
            .style(wallet_address_label_button_style),
        text(tooltip_label)
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

fn wallet_address_segments(address: String) -> Element<'static, Message> {
    let segments = Row::new()
        .push(wallet_action_segment(
            COPY_ICON_SVG,
            Message::CopyToClipboard(address.clone()),
        ))
        .push(wallet_action_separator())
        .push(wallet_action_segment(
            DETACH_ICON_SVG,
            Message::OpenWalletDetailsWindow(address.clone().into()),
        ))
        .push(wallet_action_separator())
        .push(wallet_action_segment(
            GHOST_ICON_SVG,
            Message::GhostWallet(address.into()),
        ))
        .width(Fill)
        .height(Length::Fixed(WALLET_ACTION_CELL_HEIGHT))
        .align_y(iced::Alignment::Center);

    // A single unified pill holds the icons with hairline dividers between
    // them, rather than three separate bordered buttons.
    container(segments)
        .width(Fill)
        .height(Length::Fixed(WALLET_ACTION_CELL_HEIGHT))
        .style(wallet_action_group_style)
        .into()
}

fn wallet_action_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.22,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(Length::Fixed(WALLET_ACTION_SEPARATOR_HEIGHT))
    .center_y(Fill)
    .into()
}

fn wallet_action_segment(icon_svg: &'static [u8], message: Message) -> Element<'static, Message> {
    let icon = svg(SvgHandle::from_memory(icon_svg))
        .width(Length::Fixed(WALLET_ACTION_ICON_SIZE))
        .height(Length::Fixed(WALLET_ACTION_ICON_SIZE))
        .style(move |theme: &Theme, status| {
            // Muted at rest, brightening to the accent color on hover -- no
            // jarring fill/inversion.
            let color = match status {
                svg::Status::Hovered => theme.palette().primary,
                _ => theme.extended_palette().background.weak.text,
            };
            svg::Style { color: Some(color) }
        });

    button(
        container(icon)
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill),
    )
    .on_press(message)
    .padding(0)
    .width(Length::FillPortion(1))
    .height(Length::Fixed(WALLET_ACTION_CELL_HEIGHT))
    .style(wallet_action_segment_style)
    .into()
}

fn wallet_address_label_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(
            Color {
                a: 0.18,
                ..theme.extended_palette().background.weak.color
            }
            .into(),
        ),
        _ => None,
    };

    button::Style {
        background,
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn wallet_action_group_style(theme: &Theme) -> container::Style {
    // The unified pill: a single soft surface with one hairline border.
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

fn wallet_action_segment_style(theme: &Theme, status: button::Status) -> button::Style {
    // Flat at rest; a gentle accent-tinted wash on hover signals the target
    // without the harshness of a solid fill.
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if hovered {
        Some(
            Color {
                a: 0.16,
                ..theme.palette().primary
            }
            .into(),
        )
    } else {
        None
    };

    button::Style {
        background,
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
