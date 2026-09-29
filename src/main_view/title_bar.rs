#[cfg(any(target_os = "linux", target_os = "macos"))]
mod controls;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod icons;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod resize;
#[cfg(all(test, target_os = "linux"))]
mod tests;

use crate::app_state::TradingTerminal;
use crate::message::Message;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use controls::chrome_toggle_button;
#[cfg(target_os = "linux")]
use controls::{WindowButtonKind, window_chrome_button};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use iced::mouse;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use iced::widget::{Space, column, container, mouse_area, row};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use iced::{Alignment, Fill, Length};
use iced::{Element, window};
#[cfg(target_os = "linux")]
use icons::{CLOSE_ICON_SVG, MAXIMIZE_ICON_SVG, MINIMIZE_ICON_SVG};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use icons::{
    NOTIFICATIONS_OFF_ICON_SVG, NOTIFICATIONS_ON_ICON_SVG, PNL_HIDDEN_ICON_SVG,
    PNL_VISIBLE_ICON_SVG, SOUND_OFF_ICON_SVG, SOUND_ON_ICON_SVG,
};

// ---------------------------------------------------------------------------
// Main Window Title Bar
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "linux", target_os = "macos"))]
const TITLE_BAR_HEIGHT: f32 = 34.0;
#[cfg(target_os = "macos")]
const MACOS_TRAFFIC_LIGHT_SPACER: f32 = 72.0;

impl TradingTerminal {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) fn view_main_window(&self, window_id: window::Id) -> Element<'_, Message> {
        if !self.custom_window_chrome_active {
            if !self.app_onboarding_dismissed {
                return self.view_onboarding();
            }
            return self.view_main();
        }

        let content = if !self.app_onboarding_dismissed {
            self.view_onboarding_with_top_bar(self.view_window_title_bar(window_id, false))
        } else {
            self.view_main_with_top_bar(self.view_main_chrome_header(window_id))
        };
        self.view_with_resize_handles(window_id, content)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(crate) fn view_main_window(&self, _window_id: window::Id) -> Element<'_, Message> {
        if !self.app_onboarding_dismissed {
            return self.view_onboarding();
        }

        self.view_main()
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
                .style(|theme| {
                    crate::account_views::account_summary_bar_style(
                        theme,
                        self.pane_dividers_enabled,
                    )
                }),
            content
        ]
        .width(Fill)
        .height(Fill)
        .into();

        self.view_with_resize_handles(window_id, framed_content)
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
        .style(|theme| {
            crate::account_views::account_summary_bar_style(theme, self.pane_dividers_enabled)
        })
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
}
