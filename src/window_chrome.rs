use iced::{Point, window};

#[cfg(target_os = "linux")]
const APP_ID: &str = "kerosene";
const ICON_PNG: &[u8] = include_bytes!("../assets/kerosene.png");

pub(crate) const fn custom_chrome_supported() -> bool {
    cfg!(any(target_os = "linux", target_os = "macos"))
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

pub(crate) fn settings(custom_window_chrome_enabled: bool) -> window::Settings {
    with_app_identity(window::Settings::default(), custom_window_chrome_enabled)
}

fn with_app_identity(
    mut settings: window::Settings,
    custom_window_chrome_enabled: bool,
) -> window::Settings {
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
}
