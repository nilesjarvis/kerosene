use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, row, text};
use iced::{Color, Element, Theme};

impl TradingTerminal {
    pub(super) fn view_journal_title(&self) -> Element<'static, Message> {
        let theme = self.theme();
        let title = text("Trading Journal")
            .size(20)
            .style(|theme: &Theme| text::Style {
                color: Some(theme.palette().text),
            });
        let active_profile = self.accounts.get(self.active_account_index);
        let account_label = active_profile
            .map(|profile| profile.name.as_str())
            .unwrap_or("No account");
        let active_profile_address = active_profile
            .map(|profile| profile.wallet_address.trim())
            .filter(|address| !address.is_empty());
        let address_label = self
            .journal
            .loaded_address
            .as_deref()
            .or(active_profile_address)
            .or(self.connected_address.as_deref())
            .map(|address| {
                let display = self.wallet_display(address);
                if display.has_label {
                    format!("{} ({})", display.primary, display.secondary)
                } else {
                    display.primary
                }
            })
            .unwrap_or_else(|| "Disconnected".to_string());

        let mode = if self.active_account_is_ghost() {
            JournalAccountMode::Ghost
        } else if self.active_account_can_trade() {
            JournalAccountMode::Trading
        } else {
            JournalAccountMode::WatchOnly
        };
        let muted = theme.extended_palette().background.weak.text;

        Column::new()
            .spacing(4)
            .push(
                row![title, journal_mode_badge(mode)]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
            )
            .push(
                text(format!("{} | {}", account_label, address_label))
                    .size(11)
                    .color(muted),
            )
            .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JournalAccountMode {
    Trading,
    WatchOnly,
    Ghost,
}

impl JournalAccountMode {
    fn label(self) -> &'static str {
        match self {
            JournalAccountMode::Trading => "Trading",
            JournalAccountMode::WatchOnly => "Watch only",
            JournalAccountMode::Ghost => "Ghost",
        }
    }

    fn accent(self, theme: &Theme) -> Color {
        match self {
            JournalAccountMode::Trading => theme.palette().success,
            JournalAccountMode::WatchOnly => theme.extended_palette().background.weak.text,
            JournalAccountMode::Ghost => theme.palette().primary,
        }
    }
}

fn journal_mode_badge(mode: JournalAccountMode) -> Element<'static, Message> {
    container(
        text(mode.label())
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .style(move |theme: &Theme| text::Style {
                color: Some(mode.accent(theme)),
            }),
    )
    .padding([2, 8])
    .style(move |theme: &Theme| {
        let accent = mode.accent(theme);
        container_style::Style {
            background: Some(Color { a: 0.12, ..accent }.into()),
            border: iced::Border {
                color: Color { a: 0.38, ..accent },
                width: 1.0,
                radius: 999.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}
