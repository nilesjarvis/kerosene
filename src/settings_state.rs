#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SettingsTab {
    #[default]
    Themes,
    Layouts,
    Risk,
    Integrations,
    Storage,
    Hotkeys,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ThemeSettingsPage {
    #[default]
    Overview,
    WidgetChrome,
    Crosshair,
    Notifications,
    Fonts,
    BuiltInThemes,
    CustomThemes,
}
