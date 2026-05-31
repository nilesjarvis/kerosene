use iced::window;

#[cfg(target_os = "linux")]
const APP_ID: &str = "kerosene";
const ICON_PNG: &[u8] = include_bytes!("../assets/kerosene.png");

pub(crate) fn settings() -> window::Settings {
    with_app_identity(window::Settings::default())
}

fn with_app_identity(mut settings: window::Settings) -> window::Settings {
    settings.icon = window::icon::from_file_data(ICON_PNG, Some(image::ImageFormat::Png)).ok();
    apply_platform_identity(&mut settings);
    settings
}

#[cfg(target_os = "linux")]
fn apply_platform_identity(settings: &mut window::Settings) {
    settings.platform_specific.application_id = APP_ID.to_owned();
    settings.decorations = false;
}

#[cfg(target_os = "macos")]
fn apply_platform_identity(settings: &mut window::Settings) {
    settings.platform_specific.title_hidden = true;
    settings.platform_specific.titlebar_transparent = true;
    settings.platform_specific.fullsize_content_view = true;
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn apply_platform_identity(_settings: &mut window::Settings) {}
