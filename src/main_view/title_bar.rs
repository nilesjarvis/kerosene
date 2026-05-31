use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::mouse;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Space, column, container, mouse_area, row, svg};
#[cfg(target_os = "linux")]
use iced::widget::{button, stack, text, tooltip};
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
#[cfg(target_os = "linux")]
const WINDOW_BUTTON_ICON_SIZE: f32 = 12.0;

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
        let content = self.view_main_with_top_bar(self.view_main_chrome_header(window_id));

        stack![content, self.view_window_resize_handles(window_id)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn view_main_window(&self, window_id: window::Id) -> Element<'_, Message> {
        self.view_main_with_top_bar(self.view_main_chrome_header(window_id))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(crate) fn view_main_window(&self, _window_id: window::Id) -> Element<'_, Message> {
        self.view_main()
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn view_window_chrome<'a>(
        &'a self,
        window_id: window::Id,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let framed_content: Element<'a, Message> = column![
            container(self.view_window_title_bar(window_id))
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
        column![
            container(self.view_window_title_bar(window_id))
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
                self.view_window_title_bar(window_id),
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
    fn view_window_title_bar(&self, window_id: window::Id) -> Element<'_, Message> {
        let drag_region = mouse_area(
            container(
                svg(SvgHandle::from_memory(include_bytes!(
                    "../../assets/kerosene.svg"
                )))
                .width(Length::Fixed(18.0))
                .height(Length::Fixed(18.0)),
            )
            .width(Fill)
            .height(Length::Fixed(TITLE_BAR_HEIGHT))
            .padding([0, 12])
            .align_y(Alignment::Center),
        )
        .on_press(Message::WindowDrag(window_id))
        .interaction(mouse::Interaction::Grab);

        #[cfg(target_os = "linux")]
        let title_bar = row![
            drag_region,
            window_chrome_button(
                MINIMIZE_ICON_SVG,
                "Minimize",
                Message::WindowMinimize(window_id),
                WindowButtonKind::Default,
            ),
            window_chrome_button(
                MAXIMIZE_ICON_SVG,
                "Maximize",
                Message::WindowToggleMaximize(window_id),
                WindowButtonKind::Default,
            ),
            window_chrome_button(
                CLOSE_ICON_SVG,
                "Close",
                Message::WindowClose(window_id),
                WindowButtonKind::Close,
            )
        ]
        .align_y(Alignment::Center)
        .width(Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT));

        #[cfg(target_os = "macos")]
        let title_bar = row![
            Space::new().width(Length::Fixed(MACOS_TRAFFIC_LIGHT_SPACER)),
            drag_region
        ]
        .align_y(Alignment::Center)
        .width(Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT));

        container(title_bar)
            .width(Fill)
            .height(Length::Fixed(TITLE_BAR_HEIGHT))
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
