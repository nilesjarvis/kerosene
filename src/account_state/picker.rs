use super::types::AccountPickerOption;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{container, text};
use iced::{Color, Element, Theme, color};

impl TradingTerminal {
    pub(crate) fn account_picker_options(&self) -> Vec<AccountPickerOption> {
        self.accounts
            .iter()
            .enumerate()
            .map(|(index, profile)| AccountPickerOption {
                index,
                label: profile.name.clone(),
                address: profile.wallet_address.clone(),
                can_trade: !self.ghost_account_secret_ids.contains(&profile.secret_id)
                    && Self::account_can_trade(profile),
                is_ghost: self.ghost_account_secret_ids.contains(&profile.secret_id),
            })
            .collect()
    }

    pub(crate) fn account_mode_visual(
        is_ghost: bool,
        can_trade: bool,
        theme: &Theme,
    ) -> (&'static str, Color) {
        if is_ghost {
            ("GHOST", color!(0xbd93f9))
        } else if can_trade {
            ("TRADING", theme.palette().success)
        } else {
            ("WATCH ONLY", color!(0xffb86c))
        }
    }

    pub(crate) fn account_mode_tag(
        is_ghost: bool,
        can_trade: bool,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let (label, color) = Self::account_mode_visual(is_ghost, can_trade, theme);
        container(text(label).size(9).color(color))
            .padding([2, 6])
            .style(move |_theme: &Theme| container_style::Style {
                background: Some(Color { a: 0.14, ..color }.into()),
                border: iced::Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: Color { a: 0.6, ..color },
                },
                ..Default::default()
            })
            .into()
    }

    pub(crate) fn account_integration_tag(
        label: &'static str,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let color = theme.palette().primary;
        container(text(label).size(9).color(color))
            .padding([2, 6])
            .style(move |_theme: &Theme| container_style::Style {
                background: Some(Color { a: 0.14, ..color }.into()),
                border: iced::Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: Color { a: 0.6, ..color },
                },
                ..Default::default()
            })
            .into()
    }

    pub(crate) fn account_picker_label(option: &AccountPickerOption) -> String {
        let label = option.label.trim();
        if label.is_empty() {
            format!("Account {}", option.index + 1)
        } else {
            label.to_string()
        }
    }

    pub(crate) fn truncate_display_text(value: &str, max_chars: usize) -> String {
        let char_count = value.chars().count();
        if char_count <= max_chars {
            return value.to_string();
        }
        if max_chars <= 3 {
            return value.chars().take(max_chars).collect();
        }
        let prefix: String = value.chars().take(max_chars - 3).collect();
        format!("{prefix}...")
    }

    pub(crate) fn account_picker_address_line(option: &AccountPickerOption) -> String {
        if option.address.trim().is_empty() {
            "No wallet connected".to_string()
        } else {
            Self::short_address(&option.address)
        }
    }
}
