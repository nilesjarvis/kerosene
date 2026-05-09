mod deprecated;
mod hotkeys;
mod integrations;
mod layouts;
mod risk;
mod sidebar;
mod storage;
mod themes;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::settings_state::SettingsTab;
use iced::widget::{container, row, rule};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Settings window views
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_settings_deprecated(&self) -> Element<'_, Message> {
        self.view_settings_deprecated_placeholder()
    }

    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        let right_content: Element<'_, Message> = match self.settings_active_tab {
            SettingsTab::Themes => self.view_settings_themes_section(),
            SettingsTab::Layouts => self.view_settings_layouts_section(),
            SettingsTab::Risk => self.view_settings_risk_section(),
            SettingsTab::Integrations => self.view_settings_integrations_section(),
            SettingsTab::Storage => self.view_settings_storage_section(),
            SettingsTab::Hotkeys => self.view_settings_hotkeys_section(),
        };

        let content: Element<'_, Message> = row![
            self.view_settings_sidebar(),
            container(rule::vertical(1)).height(Fill),
            container(right_content).width(Fill).height(Fill)
        ]
        .spacing(20)
        .into();

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(20)
            .into()
    }
}
