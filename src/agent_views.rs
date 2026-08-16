use crate::agent_state::{AgentChatEntry, AgentChatRole, AgentStatus, agent_tool_presentation};
use crate::app_fonts;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::openrouter_api::OpenRouterModel;

use iced::widget::container as container_style;
use iced::widget::{
    Column, Space, button, column, container, markdown, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Fill, Length, Padding, Theme};

const MAX_VISIBLE_MODEL_RESULTS: usize = 80;

// ---------------------------------------------------------------------------
// Kerosene Assistant View
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
                text("Kerosene Assistant · Pi · OpenRouter")
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
            for entry in &self.agent.entries {
                messages = messages.push(agent_entry(entry, &theme));
            }
            messages.into()
        };

        let scroll = scrollable(container(conversation).width(Fill).padding([16, 10]))
            .id(iced::widget::Id::new("kerosene-agent-chat"))
            .width(Fill)
            .height(Fill);

        let status_detail: Element<'_, Message> = if let Some(detail) = &self.agent.status_detail {
            let color = if self.agent.status == AgentStatus::Error {
                theme.palette().danger
            } else {
                theme.extended_palette().background.weak.text
            };
            container(text(detail).size(11).color(color))
                .padding([5, 8])
                .width(Fill)
                .into()
        } else {
            Space::new().height(Length::Fixed(0.0)).into()
        };

        let can_send = self.openrouter_configured()
            && !self.agent.status.is_busy()
            && !self.agent.input.trim().is_empty();
        let input = text_input(
            "Ask about your portfolio, positions, or markets…",
            &self.agent.input,
        )
        .style(helpers::text_input_style)
        .on_input(|value| Message::AgentInputChanged(value.into()))
        .on_submit_maybe(can_send.then_some(Message::AgentSubmit))
        .padding([10, 12])
        .size(13)
        .width(Fill);

        let action = if self.agent.status == AgentStatus::Thinking {
            button(text("Stop").size(12))
                .padding([10, 16])
                .on_press(Message::AgentAbort)
        } else {
            button(text("Send").size(12))
                .padding([10, 16])
                .on_press_maybe(can_send.then_some(Message::AgentSubmit))
        };

        let requested_model = self.openrouter_model_for_task();
        let (runtime_model, context_tokens, context_window) =
            self.agent.context_metrics_for_model(&requested_model);
        let display_model = runtime_model.unwrap_or(&requested_model);
        let mut context_and_usage = context_usage_summary(context_tokens, context_window);
        if let Some(usage) = api_usage_summary(self.agent.total_tokens, self.agent.total_cost_usd) {
            context_and_usage.push_str(" · ");
            context_and_usage.push_str(&usage);
        }
        let model_picker_caret = if self.agent.model_picker_open {
            "▴"
        } else {
            "▾"
        };
        let model_button = button(
            row![
                text(format!("Model · {display_model}")).size(10),
                text(model_picker_caret).size(9),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        )
        .padding([3, 5])
        .on_press_maybe(
            self.openrouter_configured()
                .then_some(Message::AgentToggleModelPicker),
        )
        .style(agent_model_footer_button_style);
        let footer = row![
            text("Read-only data access")
                .size(10)
                .color(theme.palette().success),
            Space::new().width(Fill),
            column![
                model_button,
                text(context_and_usage)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(2)
            .align_x(Alignment::End),
        ]
        .align_y(Alignment::Center);

        let configure: Element<'_, Message> = if self.openrouter_configured() {
            Space::new().height(Length::Fixed(0.0)).into()
        } else {
            button(text("Configure OpenRouter").size(11))
                .padding([7, 10])
                .on_press(Message::OpenIntegrationsSettings)
                .into()
        };

        let mut composer = Column::new()
            .push(status_detail)
            .push(configure)
            .push(row![input, action].spacing(8).align_y(Alignment::Center))
            .spacing(7);
        if self.agent.model_picker_open {
            composer = composer.push(self.view_agent_model_picker(&requested_model, &theme));
        }
        composer = composer.push(footer);

        let main_panel = container(
            column![
                container(header).padding([14, 16]),
                rule::horizontal(1),
                scroll,
                rule::horizontal(1),
                container(composer).padding([12, 16]),
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

    fn view_agent_session_sidebar(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let can_change_session = !self.agent.status.is_busy();
        let new_session = button(
            row![text("+").size(15), text("New session").size(12)]
                .spacing(7)
                .align_y(Alignment::Center),
        )
        .padding([8, 10])
        .width(Fill)
        .on_press_maybe(can_change_session.then_some(Message::AgentNewChat));

        let mut sessions = Column::new().spacing(4).width(Fill);
        for item in self.agent.session_items() {
            let count = match item.message_count {
                0 => "No messages".to_string(),
                1 => "1 message".to_string(),
                count => format!("{count} messages"),
            };
            let content = column![
                text(item.title).size(12).color(theme.palette().text),
                text(count)
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(3)
            .width(Fill);
            let active = item.active;
            sessions = sessions.push(
                button(content)
                    .padding([9, 10])
                    .width(Fill)
                    .on_press_maybe(
                        (can_change_session && !active)
                            .then_some(Message::AgentSelectSession(item.id)),
                    )
                    .style(move |theme, status| agent_session_button_style(theme, status, active)),
            );
        }

        let persistence_status: Element<'_, Message> =
            if let Some(error) = &self.agent.persistence_error {
                text(error).size(9).color(theme.palette().danger).into()
            } else if self.agent.persistence_in_flight || self.agent.persistence_dirty {
                text("Saving locally…")
                    .size(9)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            } else {
                text("Saved locally")
                    .size(9)
                    .color(theme.palette().success)
                    .into()
            };

        container(
            column![
                text("SESSIONS")
                    .size(10)
                    .font(app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text),
                new_session,
                rule::horizontal(1),
                scrollable(sessions).height(Fill),
                persistence_status,
            ]
            .spacing(10)
            .height(Fill),
        )
        .width(Length::Fixed(220.0))
        .height(Fill)
        .padding([14, 12])
        .style(agent_session_sidebar_style)
        .into()
    }

    fn view_agent_empty_state(&self) -> Element<'_, Message> {
        let theme = self.theme();
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
                text("Financial account data in the snapshot is sent to the selected OpenRouter model when needed. Keys and wallet addresses are never included.")
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

    fn view_agent_model_picker<'a>(
        &'a self,
        selected_model: &str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let query = self.agent.model_search.trim().to_lowercase();
        let mut matches = self
            .agent
            .model_catalog
            .iter()
            .filter(|model| {
                query.is_empty()
                    || model.id.to_lowercase().contains(&query)
                    || model.name.to_lowercase().contains(&query)
            })
            .collect::<Vec<_>>();
        if query.is_empty()
            && let Some(index) = matches.iter().position(|model| model.id == selected_model)
        {
            matches.swap(0, index);
        }
        let matched_count = matches.len();

        let catalog_status = if self.agent.model_catalog_loading {
            "Refreshing OpenRouter catalog…".to_string()
        } else if self.agent.model_catalog.is_empty() {
            "OpenRouter model catalog".to_string()
        } else {
            format!(
                "{} tool-capable models",
                self.agent.model_catalog.len().saturating_sub(1)
            )
        };
        let refresh = button(text("Refresh").size(10))
            .padding([5, 9])
            .on_press_maybe(
                (!self.agent.model_catalog_loading).then_some(Message::AgentRefreshModels),
            );
        let close = button(text("×").size(13))
            .padding([4, 8])
            .on_press(Message::AgentToggleModelPicker);

        let header = row![
            column![
                text("Choose Assistant model")
                    .size(12)
                    .color(theme.palette().text),
                text(format!("Current · {selected_model} · {catalog_status}"))
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(2)
            .width(Fill),
            refresh,
            close,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let search = text_input("Search by model or provider…", &self.agent.model_search)
            .id(iced::widget::Id::new("kerosene-agent-model-search"))
            .style(helpers::text_input_style)
            .on_input(Message::AgentModelSearchChanged)
            .padding([7, 9])
            .size(11)
            .width(Fill);

        let results: Element<'a, Message> = if self.agent.model_catalog.is_empty() {
            let (message, color) = if let Some(error) = &self.agent.model_catalog_error {
                (error.as_str(), theme.palette().danger)
            } else {
                (
                    "Loading tool-capable models and current pricing from OpenRouter…",
                    theme.extended_palette().background.weak.text,
                )
            };
            container(text(message).size(10).color(color))
                .center_x(Fill)
                .padding(18)
                .into()
        } else {
            let can_select = !self.agent.status.is_busy();
            let mut rows = Column::new().spacing(4).width(Fill);
            for model in matches.into_iter().take(MAX_VISIBLE_MODEL_RESULTS) {
                rows = rows.push(agent_model_option(model, selected_model, can_select, theme));
            }
            if matched_count == 0 {
                rows = rows.push(
                    container(
                        text("No tool-capable models match that search.")
                            .size(10)
                            .color(theme.extended_palette().background.weak.text),
                    )
                    .center_x(Fill)
                    .padding(18),
                );
            }
            scrollable(rows)
                .height(Length::Fixed(190.0))
                .width(Fill)
                .into()
        };

        let result_status = if matched_count > MAX_VISIBLE_MODEL_RESULTS {
            format!(
                "Showing {} of {matched_count} matches · refine the search to see more",
                MAX_VISIBLE_MODEL_RESULTS
            )
        } else if !self.agent.model_catalog.is_empty() {
            format!("{matched_count} matches")
        } else {
            String::new()
        };

        let mut content = Column::new()
            .push(header)
            .push(search)
            .push(results)
            .spacing(7);
        if let Some(error) = &self.agent.model_catalog_error
            && !self.agent.model_catalog.is_empty()
        {
            content = content.push(text(error).size(9).color(theme.palette().danger));
        }
        content = content.push(
            row![
                text(result_status)
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
                Space::new().width(Fill),
                text("Current OpenRouter rates; conditional/provider pricing may vary")
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );

        container(content)
            .padding([10, 11])
            .width(Fill)
            .style(agent_model_picker_style)
            .into()
    }
}

fn agent_model_option<'a>(
    model: &'a OpenRouterModel,
    selected_model: &str,
    can_select: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let selected = model.id == selected_model;
    let selected_label = if selected { "SELECTED" } else { "" };
    let content = column![
        row![
            text(model.name.as_str())
                .size(11)
                .color(theme.palette().text)
                .width(Fill),
            text(selected_label).size(8).color(theme.palette().primary),
        ]
        .align_y(Alignment::Center),
        text(model.id.as_str())
            .size(9)
            .color(theme.extended_palette().background.weak.text),
        row![
            text(model.pricing_summary())
                .size(9)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill),
            text(model.context_summary())
                .size(9)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(2)
    .width(Fill);

    button(content)
        .padding([7, 9])
        .width(Fill)
        .on_press_maybe(
            (can_select && !selected).then(|| Message::OpenRouterModelChanged(model.id.clone())),
        )
        .style(move |theme, status| agent_model_option_style(theme, status, selected))
        .into()
}

fn context_usage_summary(context_tokens: Option<u64>, context_window: Option<u64>) -> String {
    match (context_tokens, context_window.filter(|window| *window > 0)) {
        (Some(tokens), Some(window)) => format!(
            "Context · {} / {} ({:.1}%)",
            compact_token_count(tokens),
            compact_token_count(window),
            (tokens as f64 / window as f64) * 100.0
        ),
        (None, Some(window)) => format!("Context · — / {}", compact_token_count(window)),
        (Some(tokens), None) => format!("Context · {} / —", compact_token_count(tokens)),
        (None, None) => "Context · — / —".to_string(),
    }
}

fn api_usage_summary(total_tokens: Option<u64>, total_cost_usd: Option<f64>) -> Option<String> {
    match (total_tokens, total_cost_usd) {
        (Some(tokens), Some(cost)) => Some(format!(
            "API usage · {} tokens · ${cost:.4}",
            compact_token_count(tokens)
        )),
        (Some(tokens), None) => Some(format!(
            "API usage · {} tokens",
            compact_token_count(tokens)
        )),
        (None, Some(cost)) => Some(format!("API usage · ${cost:.4}")),
        (None, None) => None,
    }
}

fn compact_token_count(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn agent_entry<'a>(entry: &'a AgentChatEntry, theme: &Theme) -> Element<'a, Message> {
    match entry {
        AgentChatEntry::Message {
            role,
            text: body,
            markdown: markdown_content,
        } => {
            let label = match role {
                AgentChatRole::User => "You",
                AgentChatRole::Assistant => "Assistant",
            };
            let body: Element<'a, Message> = match (role, markdown_content) {
                (AgentChatRole::Assistant, Some(content)) => {
                    markdown::view(content.items(), agent_markdown_settings(theme))
                        .map(|uri| Message::AgentOpenLink(uri.into()))
                }
                _ => text(body.as_str())
                    .size(13)
                    .color(theme.palette().text)
                    .into(),
            };
            let bubble = container(
                column![
                    text(label)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                    body,
                ]
                .spacing(5),
            )
            .padding([10, 12])
            .style(match role {
                AgentChatRole::User => user_bubble_style,
                AgentChatRole::Assistant => assistant_bubble_style,
            });

            match role {
                AgentChatRole::User => {
                    row![Space::new().width(Fill), bubble.max_width(580.0)].into()
                }
                AgentChatRole::Assistant => bubble.width(Fill).max_width(660.0).into(),
            }
        }
        AgentChatEntry::Tool {
            name,
            detail,
            finished,
            is_error,
            ..
        } => {
            let presentation = agent_tool_presentation(name);
            let (icon, status, color) = if *is_error {
                ("×", "Failed", theme.palette().danger)
            } else if *finished {
                ("✓", "Complete", theme.palette().success)
            } else {
                ("…", "Running", theme.palette().warning)
            };
            let status_chip = container(
                row![
                    text(icon).size(12).color(color),
                    text(status).size(9).color(color),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .padding([3, 7])
            .style(move |theme: &Theme| agent_tool_status_style(theme, color));

            let mut content = column![
                row![
                    text(presentation.category.to_uppercase())
                        .size(9)
                        .color(theme.palette().primary),
                    Space::new().width(Fill),
                    status_chip,
                ]
                .align_y(Alignment::Center),
                text(presentation.title)
                    .size(12)
                    .color(theme.palette().text),
            ]
            .spacing(4);
            if let Some(detail) = detail {
                content = content.push(
                    text(detail.as_str())
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                );
            }

            container(content)
                .padding([9, 11])
                .width(Fill)
                .max_width(420.0)
                .style(move |theme: &Theme| agent_tool_style(theme, color))
                .into()
        }
    }
}

fn agent_markdown_settings(theme: &Theme) -> markdown::Settings {
    let mut inline_background = theme.extended_palette().background.strong.color;
    inline_background.a = 0.55;

    let style = markdown::Style {
        inline_code_highlight: markdown::Highlight {
            background: inline_background.into(),
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
        },
        inline_code_padding: Padding::from([1, 4]),
        inline_code_color: theme.palette().text,
        inline_code_font: app_fonts::monospace_font(),
        code_block_font: app_fonts::monospace_font(),
        link_color: theme.palette().primary,
        ..markdown::Style::from(theme)
    };

    let mut settings = markdown::Settings::with_text_size(13, style);
    settings.h1_size = 20.0.into();
    settings.h2_size = 18.0.into();
    settings.h3_size = 16.0.into();
    settings.h4_size = 15.0.into();
    settings.h5_size = 14.0.into();
    settings.h6_size = 13.0.into();
    settings.code_size = 12.0.into();
    settings.spacing = 8.0.into();
    settings
}

fn chip_style(theme: &Theme, color: Color) -> container_style::Style {
    let mut background = color;
    background.a = 0.08;
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            radius: 99.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

fn user_bubble_style(theme: &Theme) -> container_style::Style {
    let mut background = theme.palette().primary;
    background.a = 0.12;
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.palette().primary,
        },
        ..Default::default()
    }
}

fn assistant_bubble_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

fn agent_tool_style(_theme: &Theme, status_color: Color) -> container_style::Style {
    let mut background = status_color;
    background.a = 0.045;
    let mut border = status_color;
    border.a = 0.28;
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: border,
        },
        ..Default::default()
    }
}

fn agent_tool_status_style(theme: &Theme, color: Color) -> container_style::Style {
    let mut background = color;
    background.a = 0.08;
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            radius: 99.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

fn agent_session_sidebar_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        ..Default::default()
    }
}

fn agent_session_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = if active {
        theme.palette().primary
    } else {
        theme.palette().text
    };
    background.a = if active {
        0.13
    } else if hovered {
        0.06
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 6.0.into(),
            width: if active { 1.0 } else { 0.0 },
            color: if active {
                theme.palette().primary
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
}

fn agent_model_footer_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut background = theme.palette().primary;
    background.a = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        0.1
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 5.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_model_picker_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

fn agent_model_option_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = if selected {
        theme.palette().primary
    } else {
        theme.palette().text
    };
    background.a = if selected {
        0.12
    } else if hovered {
        0.055
    } else {
        0.0
    };
    let mut border_color = theme.palette().primary;
    border_color.a = if selected { 0.55 } else { 0.0 };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    }
}

fn agent_empty_card_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_summary_shows_used_available_and_percentage() {
        assert_eq!(
            context_usage_summary(Some(12_500), Some(200_000)),
            "Context · 12.5K / 200.0K (6.2%)"
        );
        assert_eq!(
            context_usage_summary(None, Some(1_000_000)),
            "Context · — / 1.0M"
        );
        assert_eq!(context_usage_summary(None, None), "Context · — / —");
    }

    #[test]
    fn compact_token_counts_keep_footer_readable() {
        assert_eq!(compact_token_count(999), "999");
        assert_eq!(compact_token_count(1_250), "1.2K");
        assert_eq!(compact_token_count(2_000_000), "2.0M");
    }
}
