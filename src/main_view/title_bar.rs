use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::mouse;
#[cfg(target_os = "linux")]
use iced::widget::stack;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Space, column, container, mouse_area, row, svg};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use iced::widget::{button, text, tooltip};
use iced::{Alignment, Element, Fill, Length, window};

// ---------------------------------------------------------------------------
// Main Window Title Bar
// ---------------------------------------------------------------------------

const TITLE_BAR_HEIGHT: f32 = 34.0;
#[cfg(target_os = "macos")]
const MACOS_TRAFFIC_LIGHT_SPACER: f32 = 72.0;
#[cfg(target_os = "linux")]
const RESIZE_EDGE_THICKNESS: f32 = 6.0;
#[cfg(target_os = "linux")]
const RESIZE_CORNER_SIZE: f32 = 16.0;
#[cfg(target_os = "linux")]
const WINDOW_BUTTON_WIDTH: f32 = 42.0;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const CHROME_TOGGLE_BUTTON_WIDTH: f32 = 34.0;
#[cfg(target_os = "linux")]
const WINDOW_BUTTON_ICON_SIZE: f32 = 12.0;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const CHROME_TOGGLE_ICON_SIZE: f32 = 13.0;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const SOUND_ON_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M11 5 6 9H3v6h3l5 4V5z"/>
  <path d="M15.5 8.5a5 5 0 0 1 0 7"/>
  <path d="M18.5 5.5a9 9 0 0 1 0 13"/>
</svg>
"#;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const SOUND_OFF_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M11 5 6 9H3v6h3l5 4V5z"/>
  <path d="m16 9 5 5"/>
  <path d="m21 9-5 5"/>
</svg>
"#;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const NOTIFICATIONS_ON_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9"/>
  <path d="M10 21h4"/>
</svg>
"#;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const NOTIFICATIONS_OFF_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
  <path d="M18.63 13A17.9 17.9 0 0 1 18 8a6 6 0 0 0-9.33-5"/>
  <path d="M6.26 6.26A6 6 0 0 0 6 8c0 7-3 7-3 9h14"/>
  <path d="m2 2 20 20"/>
</svg>
"#;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const PNL_VISIBLE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/>
  <circle cx="12" cy="12" r="3"/>
</svg>
"#;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const PNL_HIDDEN_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M9.88 9.88A3 3 0 0 0 14.12 14.12"/>
  <path d="M10.73 5.08A10.4 10.4 0 0 1 12 5c6.5 0 10 7 10 7a18.4 18.4 0 0 1-4.14 5.02"/>
  <path d="M6.61 6.61A18.4 18.4 0 0 0 2 12s3.5 7 10 7a10.7 10.7 0 0 0 4.39-.9"/>
  <path d="m2 2 20 20"/>
</svg>
"#;

#[cfg(target_os = "linux")]
const MINIMIZE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round">
  <path d="M5 12h14"/>
</svg>
"#;

#[cfg(target_os = "linux")]
const MAXIMIZE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linejoin="round">
  <rect x="6" y="6" width="12" height="12" rx="1"/>
</svg>
"#;

#[cfg(target_os = "linux")]
const CLOSE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round">
  <path d="M6 6l12 12"/>
  <path d="M18 6 6 18"/>
</svg>
"#;

impl TradingTerminal {
    #[cfg(target_os = "linux")]
    pub(crate) fn view_main_window(&self, window_id: window::Id) -> Element<'_, Message> {
        if !self.custom_window_chrome_active {
            if !self.app_onboarding_dismissed {
                return self.view_onboarding();
            }
            return self.view_main();
        }

        if !self.app_onboarding_dismissed {
            return stack![
                self.view_onboarding_with_top_bar(self.view_window_title_bar(window_id, false)),
                self.view_window_resize_handles(window_id)
            ]
            .width(Fill)
            .height(Fill)
            .into();
        }

        let content = self.view_main_with_top_bar(self.view_main_chrome_header(window_id));

        stack![content, self.view_window_resize_handles(window_id)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn view_main_window(&self, window_id: window::Id) -> Element<'_, Message> {
        if !self.custom_window_chrome_active {
            if !self.app_onboarding_dismissed {
                return self.view_onboarding();
            }
            return self.view_main();
        }

        if !self.app_onboarding_dismissed {
            return self.view_onboarding_with_top_bar(self.view_window_title_bar(window_id, false));
        }

        self.view_main_with_top_bar(self.view_main_chrome_header(window_id))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(crate) fn view_main_window(&self, _window_id: window::Id) -> Element<'_, Message> {
        if !self.app_onboarding_dismissed {
            return self.view_onboarding();
        }

        self.view_main()
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn view_window_chrome<'a>(
        &'a self,
        window_id: window::Id,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        if !self.custom_window_chrome_active {
            return content;
        }

        let framed_content: Element<'a, Message> = column![
            container(self.view_window_title_bar(window_id, false))
                .width(Fill)
                .style(crate::account_views::account_summary_bar_style),
            content
        ]
        .width(Fill)
        .height(Fill)
        .into();

        stack![framed_content, self.view_window_resize_handles(window_id)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn view_window_chrome<'a>(
        &'a self,
        window_id: window::Id,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        if !self.custom_window_chrome_active {
            return content;
        }

        column![
            container(self.view_window_title_bar(window_id, false))
                .width(Fill)
                .style(crate::account_views::account_summary_bar_style),
            content
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(crate) fn view_window_chrome<'a>(
        &'a self,
        _window_id: window::Id,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        content
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn view_main_chrome_header(&self, window_id: window::Id) -> Element<'_, Message> {
        container(
            column![
                self.view_window_title_bar(window_id, true),
                container(self.view_account_summary())
                    .width(Fill)
                    .height(Length::Fixed(self.account_summary_bar_height()))
            ]
            .width(Fill),
        )
        .width(Fill)
        .style(crate::account_views::account_summary_bar_style)
        .into()
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn view_window_title_bar(
        &self,
        window_id: window::Id,
        show_status_toggles: bool,
    ) -> Element<'_, Message> {
        let drag_region = mouse_area(
            Space::new()
                .width(Fill)
                .height(Length::Fixed(TITLE_BAR_HEIGHT)),
        )
        .on_press(Message::WindowDrag(window_id))
        .interaction(mouse::Interaction::Grab);

        #[cfg(target_os = "linux")]
        let title_bar = {
            let mut controls = row![drag_region].align_y(Alignment::Center);

            if show_status_toggles {
                controls = controls.push(self.view_chrome_status_toggles());
            }

            controls
                .push(window_chrome_button(
                    MINIMIZE_ICON_SVG,
                    "Minimize",
                    Message::WindowMinimize(window_id),
                    WindowButtonKind::Default,
                ))
                .push(window_chrome_button(
                    MAXIMIZE_ICON_SVG,
                    "Maximize",
                    Message::WindowToggleMaximize(window_id),
                    WindowButtonKind::Default,
                ))
                .push(window_chrome_button(
                    CLOSE_ICON_SVG,
                    "Close",
                    Message::WindowClose(window_id),
                    WindowButtonKind::Close,
                ))
                .width(Fill)
                .height(Length::Fixed(TITLE_BAR_HEIGHT))
        };

        #[cfg(target_os = "macos")]
        let title_bar = {
            let mut controls = row![
                Space::new().width(Length::Fixed(MACOS_TRAFFIC_LIGHT_SPACER)),
                drag_region
            ]
            .align_y(Alignment::Center);

            if show_status_toggles {
                controls = controls.push(self.view_chrome_status_toggles());
            }

            controls.width(Fill).height(Length::Fixed(TITLE_BAR_HEIGHT))
        };

        container(title_bar)
            .width(Fill)
            .height(Length::Fixed(TITLE_BAR_HEIGHT))
            .into()
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn view_chrome_status_toggles(&self) -> Element<'static, Message> {
        row![
            chrome_toggle_button(
                if self.hide_pnl {
                    PNL_HIDDEN_ICON_SVG
                } else {
                    PNL_VISIBLE_ICON_SVG
                },
                if self.hide_pnl {
                    "Show PnL"
                } else {
                    "Hide PnL"
                },
                self.hide_pnl,
                Message::ToggleHidePnl,
            ),
            chrome_toggle_button(
                if self.sound_enabled {
                    SOUND_ON_ICON_SVG
                } else {
                    SOUND_OFF_ICON_SVG
                },
                if self.sound_enabled {
                    "Mute sound"
                } else {
                    "Enable sound"
                },
                self.sound_enabled,
                Message::ToggleSound,
            ),
            chrome_toggle_button(
                if self.desktop_notifications {
                    NOTIFICATIONS_ON_ICON_SVG
                } else {
                    NOTIFICATIONS_OFF_ICON_SVG
                },
                if self.desktop_notifications {
                    "Disable desktop notifications"
                } else {
                    "Enable desktop notifications"
                },
                self.desktop_notifications,
                Message::ToggleDesktopNotifications,
            )
        ]
        .align_y(Alignment::Center)
        .into()
    }

    #[cfg(target_os = "linux")]
    fn view_window_resize_handles(&self, window_id: window::Id) -> Element<'static, Message> {
        let side_edges: Element<'static, Message> = row![
            resize_handle(
                window_id,
                window::Direction::West,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                Length::Fill,
                mouse::Interaction::ResizingHorizontally,
            ),
            Space::new().width(Fill),
            resize_handle(
                window_id,
                window::Direction::East,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                Length::Fill,
                mouse::Interaction::ResizingHorizontally,
            )
        ]
        .width(Fill)
        .height(Fill)
        .into();

        let top_bottom_edges: Element<'static, Message> = column![
            resize_handle(
                window_id,
                window::Direction::North,
                Length::Fill,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                mouse::Interaction::ResizingVertically,
            ),
            Space::new().height(Fill),
            resize_handle(
                window_id,
                window::Direction::South,
                Length::Fill,
                Length::Fixed(RESIZE_EDGE_THICKNESS),
                mouse::Interaction::ResizingVertically,
            )
        ]
        .width(Fill)
        .height(Fill)
        .into();

        let corners: Element<'static, Message> = column![
            row![
                resize_handle(
                    window_id,
                    window::Direction::NorthWest,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyDown,
                ),
                Space::new().width(Fill),
                resize_handle(
                    window_id,
                    window::Direction::NorthEast,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyUp,
                )
            ]
            .width(Fill),
            Space::new().height(Fill),
            row![
                resize_handle(
                    window_id,
                    window::Direction::SouthWest,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyUp,
                ),
                Space::new().width(Fill),
                resize_handle(
                    window_id,
                    window::Direction::SouthEast,
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    Length::Fixed(RESIZE_CORNER_SIZE),
                    mouse::Interaction::ResizingDiagonallyDown,
                )
            ]
            .width(Fill)
        ]
        .width(Fill)
        .height(Fill)
        .into();

        stack![side_edges, top_bottom_edges, corners]
            .width(Fill)
            .height(Fill)
            .into()
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowButtonKind {
    Default,
    Close,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn chrome_toggle_button(
    icon_svg: &'static [u8],
    label: &'static str,
    enabled: bool,
    message: Message,
) -> Element<'static, Message> {
    let icon: Element<'static, Message> = svg(SvgHandle::from_memory(icon_svg))
        .width(Length::Fixed(CHROME_TOGGLE_ICON_SIZE))
        .height(Length::Fixed(CHROME_TOGGLE_ICON_SIZE))
        .style(move |theme: &iced::Theme, _status| svg::Style {
            color: Some(chrome_toggle_icon_color(theme, enabled)),
        })
        .into();

    let control = button(container(icon).center_x(Fill).center_y(Fill))
        .on_press(message)
        .width(Length::Fixed(CHROME_TOGGLE_BUTTON_WIDTH))
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding(0)
        .style(move |theme: &iced::Theme, status| {
            let background = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(theme.extended_palette().background.weak.color.into())
                }
                _ => None,
            };

            button::Style {
                background,
                text_color: chrome_toggle_icon_color(theme, enabled),
                border: iced::Border {
                    radius: 0.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    tooltip(
        control,
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Bottom,
    )
    .into()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn chrome_toggle_icon_color(theme: &iced::Theme, enabled: bool) -> iced::Color {
    if enabled {
        theme.palette().success
    } else {
        theme.extended_palette().background.weak.text
    }
}

#[cfg(target_os = "linux")]
fn window_chrome_button(
    icon_svg: &'static [u8],
    label: &'static str,
    message: Message,
    kind: WindowButtonKind,
) -> Element<'static, Message> {
    let icon: Element<'static, Message> = svg(SvgHandle::from_memory(icon_svg))
        .width(Length::Fixed(WINDOW_BUTTON_ICON_SIZE))
        .height(Length::Fixed(WINDOW_BUTTON_ICON_SIZE))
        .style(|theme: &iced::Theme, _status| svg::Style {
            color: Some(theme.palette().text),
        })
        .into();

    let control = button(container(icon).center_x(Fill).center_y(Fill))
        .on_press(message)
        .width(Length::Fixed(WINDOW_BUTTON_WIDTH))
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding(0)
        .style(move |theme: &iced::Theme, status| {
            let palette = theme.extended_palette();
            let background = match (kind, status) {
                (WindowButtonKind::Close, button::Status::Hovered | button::Status::Pressed) => {
                    Some(palette.danger.strong.color.into())
                }
                (_, button::Status::Hovered | button::Status::Pressed) => {
                    Some(palette.background.weak.color.into())
                }
                _ => None,
            };

            button::Style {
                background,
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 0.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    tooltip(
        control,
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Bottom,
    )
    .into()
}

#[cfg(target_os = "linux")]
fn resize_handle(
    window_id: window::Id,
    direction: window::Direction,
    width: Length,
    height: Length,
    interaction: mouse::Interaction,
) -> Element<'static, Message> {
    mouse_area(Space::new().width(width).height(height))
        .on_press(Message::WindowDragResize(window_id, direction))
        .interaction(interaction)
        .into()
}
