use iced::{Point, window};
use std::sync::OnceLock;

#[cfg(target_os = "linux")]
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};

#[cfg(target_os = "linux")]
const APP_ID: &str = "kerosene";
const ICON_PNG: &[u8] = include_bytes!("../assets/kerosene.png");
static BACKGROUND_BLUR_SUPPORTED: OnceLock<bool> = OnceLock::new();

pub(crate) const fn custom_chrome_supported() -> bool {
    cfg!(any(target_os = "linux", target_os = "macos"))
}

pub(crate) fn background_blur_supported() -> bool {
    *BACKGROUND_BLUR_SUPPORTED.get_or_init(detect_background_blur_support)
}

pub(crate) const fn background_blur_unavailable_reason() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "This Linux compositor does not expose app-requested background blur."
    }

    #[cfg(target_os = "windows")]
    {
        "The current Windows window backend does not support background blur."
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        "Background blur is not supported in this desktop session."
    }
}

#[cfg(target_os = "macos")]
fn detect_background_blur_support() -> bool {
    true
}

#[cfg(target_os = "linux")]
fn detect_background_blur_support() -> bool {
    let Ok(connection) = Connection::connect_to_env() else {
        return false;
    };
    let Ok((globals, _event_queue)) = registry_queue_init::<BlurCapabilityProbe>(&connection)
    else {
        return false;
    };

    globals.contents().with_list(|globals| {
        wayland_interfaces_support_background_blur(
            globals.iter().map(|global| global.interface.as_str()),
        )
    })
}

#[cfg(target_os = "linux")]
fn wayland_interfaces_support_background_blur<'a>(
    interfaces: impl IntoIterator<Item = &'a str>,
) -> bool {
    interfaces
        .into_iter()
        .any(|interface| interface == "org_kde_kwin_blur_manager")
}

#[cfg(target_os = "linux")]
struct BlurCapabilityProbe;

#[cfg(target_os = "linux")]
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for BlurCapabilityProbe {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn detect_background_blur_support() -> bool {
    false
}

pub(crate) fn restored_position(point: Point) -> window::Position {
    if restored_point_is_visible(point) {
        window::Position::Specific(point)
    } else {
        window::Position::Centered
    }
}

fn restored_point_is_visible(point: Point) -> bool {
    if !point.x.is_finite() || !point.y.is_finite() {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        };

        let (left, top, width, height) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        if width <= 0 || height <= 0 {
            return false;
        }
        point.x >= left as f32
            && point.y >= top as f32
            && point.x < left.saturating_add(width).saturating_sub(64) as f32
            && point.y < top.saturating_add(height).saturating_sub(34) as f32
    }

    #[cfg(not(target_os = "windows"))]
    true
}

pub(crate) fn settings(
    custom_window_chrome_enabled: bool,
    window_background_blur_enabled: bool,
) -> window::Settings {
    with_app_identity(
        window::Settings::default(),
        custom_window_chrome_enabled,
        window_background_blur_enabled,
    )
}

fn with_app_identity(
    mut settings: window::Settings,
    custom_window_chrome_enabled: bool,
    window_background_blur_enabled: bool,
) -> window::Settings {
    // Iced does not expose a runtime transparency toggle. Creating every app
    // window with an alpha-capable native surface lets the persisted appearance
    // preference take effect immediately while opaque themes remain unchanged.
    settings.transparent = true;
    settings.blur = window_background_blur_enabled && background_blur_supported();
    settings.icon = window::icon::from_file_data(ICON_PNG, Some(image::ImageFormat::Png)).ok();
    apply_platform_identity(&mut settings, custom_window_chrome_enabled);
    settings
}

#[cfg(target_os = "linux")]
fn apply_platform_identity(settings: &mut window::Settings, custom_window_chrome_enabled: bool) {
    settings.platform_specific.application_id = APP_ID.to_owned();
    if custom_window_chrome_enabled {
        settings.decorations = false;
    }
}

#[cfg(target_os = "macos")]
fn apply_platform_identity(settings: &mut window::Settings, custom_window_chrome_enabled: bool) {
    if custom_window_chrome_enabled {
        settings.platform_specific.title_hidden = true;
        settings.platform_specific.titlebar_transparent = true;
        settings.platform_specific.fullsize_content_view = true;
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn apply_platform_identity(_settings: &mut window::Settings, _custom_window_chrome_enabled: bool) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_position_rejects_non_finite_coordinates() {
        assert!(matches!(
            restored_position(Point::new(f32::NAN, 10.0)),
            window::Position::Centered
        ));
        assert!(matches!(
            restored_position(Point::new(10.0, f32::INFINITY)),
            window::Position::Centered
        ));
    }

    #[test]
    fn app_windows_are_created_with_alpha_capable_surfaces() {
        assert!(settings(false, false).transparent);
    }

    #[test]
    fn background_blur_is_applied_only_when_enabled_and_supported() {
        assert!(!settings(false, false).blur);
        assert_eq!(settings(false, true).blur, background_blur_supported());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_blur_requires_the_compositor_protocol() {
        assert!(!wayland_interfaces_support_background_blur([
            "wl_compositor",
            "xdg_wm_base",
        ]));
        assert!(wayland_interfaces_support_background_blur([
            "wl_compositor",
            "org_kde_kwin_blur_manager",
        ]));
    }
}
