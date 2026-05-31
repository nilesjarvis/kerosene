use iced::window;

#[cfg(target_os = "linux")]
const APP_ID: &str = "kerosene";
const ICON_PNG: &[u8] = include_bytes!("../assets/kerosene.png");

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
