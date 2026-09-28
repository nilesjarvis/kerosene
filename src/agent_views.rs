use crate::agent_state::{AgentChatEntry, AgentStatus};
use crate::app_state::TradingTerminal;
use crate::config::AssistantProvider;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, button, column, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill, Theme};

mod composer;
mod conversation;
mod models;
mod sessions;
mod streaming;
mod styles;

use conversation::{agent_entry, agent_tool_trace_starts_at};
use styles::{agent_empty_card_style, agent_pnl_card_hover_style, chip_style};

// ---------------------------------------------------------------------------
// Kerosene Assistant window and welcome screen
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_agent_window(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let status_color = match self.agent.status {
            AgentStatus::Ready => theme.palette().success,
            AgentStatus::Error => theme.palette().danger,
            AgentStatus::Thinking | AgentStatus::Preparing | AgentStatus::Starting => {
                theme.palette().warning
            }
            AgentStatus::Stopped => theme.extended_palette().background.weak.text,
        };

        let status_chip = container(
            row![
                text("●").size(9).color(status_color),
                text(self.agent.status.label()).size(11).color(status_color),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([5, 9])
        .style(move |theme: &Theme| chip_style(theme, status_color));

        let header = row![
            column![
                text(self.agent.active_session_title.as_str())
                    .size(17)
                    .color(theme.palette().text),
                text(format!(
                    "Kerosene Assistant · Pi · {}",
                    self.assistant_provider.label()
                ))
                .size(11)
                .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(2)
            .width(Fill),
            status_chip,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let conversation = if self.agent.entries.is_empty() {
            self.view_agent_empty_state()
        } else {
            let mut messages = Column::new().spacing(12).width(Fill);
            for (index, entry) in self.agent.entries.iter().enumerate() {
                if matches!(entry, AgentChatEntry::Tool { .. })
                    && !agent_tool_trace_starts_at(&self.agent.entries, index)
                {
                    continue;
                }
                messages = messages.push(agent_entry(&self.agent, index, entry, &theme));
            }
            messages.into()
        };

        let scroll = scrollable(container(conversation).width(Fill).padding([16, 10]))
            .id(iced::widget::Id::new("kerosene-agent-chat"))
            .width(Fill)
            .height(Fill);

        let main_panel = container(
            column![
                container(header).padding([14, 16]),
                rule::horizontal(1),
                scroll,
                rule::horizontal(1),
                container(self.view_agent_composer(&theme)).padding([12, 16]),
            ]
            .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            ..Default::default()
        });

        container(row![
            self.view_agent_session_sidebar(),
            rule::vertical(1),
            main_panel,
        ])
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            ..Default::default()
        })
        .into()
    }

    fn view_agent_empty_state(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let attachment_privacy = match self.assistant_provider {
            AssistantProvider::OpenRouter => {
                "The image and sanitized account context are sent to the selected OpenRouter model. Keys are never included. Public wallet candidates are exposed only for an explicitly attached card turn."
            }
            AssistantProvider::LlamaCpp => {
                "The image and sanitized account context are sent to the detected llama.cpp server on this machine. Keys are never included. Public wallet candidates are exposed only for an explicitly attached card turn."
            }
        };
        container(
            column![
                text("Ask Kerosene anything")
                    .size(20)
                    .color(theme.palette().text),
                text("The assistant can inspect a fresh, sanitized snapshot of your account, portfolio, live mids, aggregate positioning, and session analytics.")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
                container(
                    column![
                        text("Try asking").size(11).color(theme.palette().primary),
                        text("• Where is my portfolio most concentrated?\n• Summarize my open-position risk.\n• Compare my active market with current positioning.")
                            .size(12)
                            .color(theme.palette().text),
                    ]
                    .spacing(7),
                )
                .padding(14)
                .width(Fill)
                .style(agent_empty_card_style),
                container(
                    row![
                        column![
                            text("Analyze a social P&L card")
                                .size(13)
                                .color(theme.palette().text),
                            text(if self.agent.pnl_card_drop_hovered {
                                "Release to attach this image"
                            } else {
                                "Drop an image here, extract the visible trade, then search public HyperDash and Hyperliquid position data."
                            })
                            .size(10)
                            .color(theme.extended_palette().background.weak.text),
                        ]
                        .spacing(4)
                        .width(Fill),
                        button(text("Choose image").size(11))
                            .padding([7, 10])
                            .on_press(Message::AgentPnlCardBrowse),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding(14)
                .width(Fill)
                .style(if self.agent.pnl_card_drop_hovered {
                    agent_pnl_card_hover_style
                } else {
                    agent_empty_card_style
                }),
                text(attachment_privacy)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(14)
            .max_width(560.0),
        )
        .center_x(Fill)
        .center_y(Fill)
        .into()
    }
}
