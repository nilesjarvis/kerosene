#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SettingsTab {
    #[default]
    Themes,
    Layouts,
    Risk,
    Integrations,
    Network,
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

#[derive(Debug, Default)]
pub(crate) struct HyperliquidProxySettings {
    pub(crate) enabled: bool,
    pub(crate) urls: Vec<crate::api::proxy::ProxyUrl>,
    pub(crate) input: crate::app_state::SensitiveString,
    pub(crate) status: Option<(String, bool)>,
}

impl HyperliquidProxySettings {
    pub(crate) fn from_config(config: &crate::config::KeroseneConfig) -> Self {
        let mut settings = Self {
            enabled: config.hyperliquid_proxies_enabled,
            urls: config.hyperliquid_proxy_urls.clone(),
            ..Self::default()
        };
        settings.apply();
        settings
    }

    pub(crate) fn apply(&mut self) {
        match crate::api::proxy::ProxyPool::build(self.enabled, &self.urls) {
            Ok(pool) => crate::api::proxy::install(pool),
            Err(error) => {
                // Keep reads paused until invalid saved credentials are corrected.
                if let Ok(pool) = crate::api::proxy::ProxyPool::build(self.enabled, &[]) {
                    crate::api::proxy::install(pool);
                }
                self.status = Some((error, true));
            }
        }
    }
}
mod builder;
pub(crate) use builder::{BuilderApprovalRequest, BuilderCodeSettings};
