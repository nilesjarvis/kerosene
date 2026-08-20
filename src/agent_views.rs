use crate::agent_state::{
    AgentChatEntry, AgentChatRole, AgentPrompt, AgentState, AgentStatus, agent_tool_presentation,
};
use crate::app_fonts;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::openrouter_api::OpenRouterModel;

use iced::widget::container as container_style;
use iced::widget::{
    Column, Space, button, column, container, image, markdown, rich_text, row, rule, scrollable,
    text, text_input,
};
use iced::{Alignment, Border, Color, ContentFit, Element, Fill, Length, Padding, Theme};

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
            for (index, entry) in self.agent.entries.iter().enumerate() {
                messages = messages.push(agent_entry(&self.agent, index, entry, &theme));
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

        let has_pnl_card = self.agent.pnl_card_attachment.is_some();
        let requested_model = self.openrouter_model_for_task();
        let image_model_ready =
            !has_pnl_card || self.agent.model_supports_images(&requested_model) == Some(true);
        let can_send = self.openrouter_configured()
            && !self.agent.status.is_busy()
            && !self.agent.pnl_card_loading
            && image_model_ready
            && (!self.agent.input.trim().is_empty() || has_pnl_card);
        let input = text_input(
            if has_pnl_card {
                "Optional: add context about this P&L card…"
            } else {
                "Ask about your portfolio, positions, or markets…"
            },
            &self.agent.input,
        )
        .id(iced::widget::Id::new("kerosene-agent-input"))
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

        let attach = button(
            text(if has_pnl_card {
                "Replace card"
            } else {
                "+ P&L card"
            })
            .size(11),
        )
        .padding([7, 10])
        .on_press_maybe(
            (!self.agent.status.is_busy() && !self.agent.pnl_card_loading)
                .then_some(Message::AgentPnlCardBrowse),
        );
        let attachment_controls = row![
            attach,
            text(if self.hyperdash_api_key.trim().is_empty() {
                "OCR works now · add HyperDash in Settings to search public positions"
            } else {
                "Drop PNG, JPEG, or WebP anywhere in this window"
            })
            .size(9)
            .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut composer = Column::new()
            .push(status_detail)
            .push(configure)
            .push(self.view_agent_pnl_card_attachment(&theme))
            .push(attachment_controls)
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
                text("The image and sanitized account context are sent to the selected OpenRouter model. Keys are never included. Public wallet candidates are exposed only for an explicitly attached card turn.")
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

    fn view_agent_pnl_card_attachment(&self, theme: &Theme) -> Element<'_, Message> {
        if self.agent.pnl_card_drop_hovered {
            return container(
                text("Release to attach this P&L card")
                    .size(11)
                    .color(theme.palette().primary),
            )
            .padding([10, 12])
            .center_x(Fill)
            .width(Fill)
            .style(agent_pnl_card_hover_style)
            .into();
        }
        if self.agent.pnl_card_loading {
            return container(
                text("Preparing P&L card…")
                    .size(10)
                    .color(theme.palette().warning),
            )
            .padding([7, 9])
            .width(Fill)
            .style(agent_empty_card_style)
            .into();
        }
        if let Some(attachment) = &self.agent.pnl_card_attachment {
            let preview = image(attachment.preview_handle.clone())
                .width(Length::Fixed(96.0))
                .height(Length::Fixed(64.0))
                .content_fit(ContentFit::Contain);
            return container(
                row![
                    preview,
                    column![
                        text(attachment.file_label.as_str())
                            .size(11)
                            .color(theme.palette().text),
                        text(format!(
                            "{} × {} · sent only with this turn",
                            attachment.width, attachment.height
                        ))
                        .size(9)
                        .color(theme.extended_palette().background.weak.text),
                        text("Vision + tools model required")
                            .size(9)
                            .color(theme.palette().primary),
                    ]
                    .spacing(3)
                    .width(Fill),
                    button(text("Remove").size(10))
                        .padding([5, 8])
                        .on_press(Message::AgentPnlCardRemove),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([8, 10])
            .width(Fill)
            .style(agent_empty_card_style)
            .into();
        }
        if let Some(error) = &self.agent.pnl_card_error {
            return container(text(error).size(10).color(theme.palette().danger))
                .padding([6, 8])
                .width(Fill)
                .into();
        }
        Space::new().height(Length::Fixed(0.0)).into()
    }

    fn view_agent_model_picker<'a>(
        &'a self,
        selected_model: &str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let query = self.agent.model_search.trim().to_lowercase();
        let vision_required = self.agent.pnl_card_attachment.is_some();
        let mut matches = self
            .agent
            .model_catalog
            .iter()
            .filter(|model| {
                (!vision_required || model.supports_image_input)
                    && (query.is_empty()
                        || model.id.to_lowercase().contains(&query)
                        || model.name.to_lowercase().contains(&query)
                        || model.provider_summary().to_lowercase().contains(&query))
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
                "{} {}models",
                matches.len(),
                if vision_required {
                    "vision + tools "
                } else {
                    "tool-capable "
                }
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
                text(if vision_required {
                    "Choose a vision + tools model"
                } else {
                    "Choose Assistant model"
                })
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
                        text(if vision_required {
                            "No vision + tools models match that search."
                        } else {
                            "No tool-capable models match that search."
                        })
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
            text(format!(
                "{} · {}",
                model.provider_summary(),
                if model.supports_image_input {
                    "Vision"
                } else {
                    "Text"
                }
            ))
            .size(9)
            .color(theme.palette().primary)
            .width(Fill),
            text(model.context_summary())
                .size(9)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(model.pricing_summary())
            .size(9)
            .color(theme.extended_palette().background.weak.text),
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

fn agent_entry<'a>(
    agent: &'a AgentState,
    entry_index: usize,
    entry: &'a AgentChatEntry,
    theme: &Theme,
) -> Element<'a, Message> {
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
            let streaming = *role == AgentChatRole::Assistant
                && agent.assistant_entry_index == Some(entry_index);
            let body: Element<'a, Message> = match (role, markdown_content) {
                (AgentChatRole::Assistant, Some(content)) if streaming => agent_streaming_markdown(
                    content,
                    agent_markdown_settings(theme),
                    theme.palette().text,
                    agent.stream.word_progress,
                    agent.stream.cursor_visible,
                ),
                (AgentChatRole::Assistant, Some(content)) => {
                    markdown::view(content.items(), agent_markdown_settings(theme))
                        .map(|uri| Message::AgentOpenLink(uri.into()))
                }
                _ => text(body.as_str())
                    .size(13)
                    .color(theme.palette().text)
                    .into(),
            };
            let mut content = Column::new()
                .push(
                    text(label)
                        .size(10)
                        .color(theme.extended_palette().background.weak.text),
                )
                .push(body)
                .spacing(5);

            let featured = *role == AgentChatRole::Assistant
                && agent.stream.featured_entry_index == Some(entry_index);
            if featured {
                let evidence = agent_turn_evidence(&agent.entries, entry_index);
                let progress = agent.stream.completion_progress;
                content = content.push(agent_response_actions(
                    entry_index,
                    evidence.len(),
                    agent.stream.evidence_open,
                    !agent.featured_response_has_image,
                    progress,
                    theme,
                ));
                if agent.stream.evidence_open && !evidence.is_empty() {
                    content = content.push(agent_evidence_drawer(&evidence, theme));
                }
                content = content.push(agent_follow_up_view(&evidence, progress, theme));
            }

            let bubble = container(content).padding([10, 12]).style(match role {
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

struct AgentEvidence<'a> {
    name: &'a str,
    detail: Option<&'a str>,
    finished: bool,
    is_error: bool,
}

fn agent_turn_evidence<'a>(
    entries: &'a [AgentChatEntry],
    entry_index: usize,
) -> Vec<AgentEvidence<'a>> {
    let Some(before_entry) = entries.get(..entry_index) else {
        return Vec::new();
    };
    let turn_start = before_entry
        .iter()
        .rposition(|entry| {
            matches!(
                entry,
                AgentChatEntry::Message {
                    role: AgentChatRole::User,
                    ..
                }
            )
        })
        .map_or(0, |index| index + 1);

    entries[turn_start..entry_index]
        .iter()
        .filter_map(|entry| match entry {
            AgentChatEntry::Tool {
                name,
                detail,
                finished,
                is_error,
                ..
            } => Some(AgentEvidence {
                name,
                detail: detail.as_deref(),
                finished: *finished,
                is_error: *is_error,
            }),
            AgentChatEntry::Message { .. } => None,
        })
        .collect()
}

fn agent_response_actions<'a>(
    entry_index: usize,
    evidence_count: usize,
    evidence_open: bool,
    can_regenerate: bool,
    progress: f32,
    theme: &Theme,
) -> Element<'a, Message> {
    let enabled = progress >= 0.72;
    let color = with_alpha(
        theme.extended_palette().background.weak.text,
        0.25 + progress * 0.75,
    );
    let copy = button(text("Copy").size(10).color(color))
        .padding([4, 7])
        .on_press_maybe(enabled.then_some(Message::AgentCopyResponse(entry_index)))
        .style(move |theme, status| agent_response_action_style(theme, status, progress));
    let retry = button(
        text(if can_regenerate {
            "Regenerate"
        } else {
            "Reattach to regenerate"
        })
        .size(10)
        .color(color),
    )
    .padding([4, 7])
    .on_press_maybe(
        (enabled && can_regenerate).then_some(Message::AgentRegenerateResponse(entry_index)),
    )
    .style(move |theme, status| agent_response_action_style(theme, status, progress));

    let mut actions = row![copy, retry].spacing(2).align_y(Alignment::Center);
    if evidence_count > 0 {
        let caret = if evidence_open { "▴" } else { "▾" };
        let evidence = button(
            text(format!("{evidence_count} data calls {caret}"))
                .size(10)
                .color(color),
        )
        .padding([4, 7])
        .on_press_maybe(enabled.then_some(Message::AgentToggleEvidence(entry_index)))
        .style(move |theme, status| agent_response_action_style(theme, status, progress));
        actions = actions.push(evidence);
    }
    actions.into()
}

fn agent_evidence_drawer<'a>(
    evidence: &[AgentEvidence<'a>],
    theme: &Theme,
) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(2).width(Fill);
    for item in evidence {
        let presentation = agent_tool_presentation(item.name);
        let (status, status_color) = if item.is_error {
            ("Failed", theme.palette().danger)
        } else if item.finished {
            ("Complete", theme.palette().success)
        } else {
            ("Running", theme.palette().warning)
        };
        let header = row![
            text(presentation.category)
                .size(9)
                .color(theme.palette().primary),
            text(presentation.title)
                .size(10)
                .color(theme.palette().text),
            Space::new().width(Fill),
            text(status).size(9).color(status_color),
        ]
        .spacing(7)
        .align_y(Alignment::Center);
        let mut content = Column::new().push(header).spacing(2).width(Fill);
        if let Some(detail) = item.detail {
            content = content.push(
                text(detail)
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            );
        }
        rows = rows.push(container(content).padding([5, 7]));
    }
    container(rows)
        .padding(4)
        .width(Fill)
        .style(agent_evidence_style)
        .into()
}

fn agent_follow_up_view<'a>(
    evidence: &[AgentEvidence<'a>],
    progress: f32,
    theme: &Theme,
) -> Element<'a, Message> {
    let follow_ups = agent_follow_ups(evidence);
    let enabled = progress >= 0.72;
    let muted = with_alpha(
        theme.extended_palette().background.weak.text,
        0.2 + progress * 0.8,
    );
    let mut rows = Column::new()
        .push(text("Follow-ups").size(10).color(muted))
        .spacing(2)
        .width(Fill);
    for (index, follow_up) in follow_ups.into_iter().enumerate() {
        let stagger = ((progress - index as f32 * 0.18) / 0.82).clamp(0.0, 1.0);
        let color = with_alpha(theme.palette().text, 0.12 + stagger * 0.88);
        rows =
            rows.push(
                button(
                    row![
                        text("↳").size(10).color(muted),
                        text(follow_up).size(11).color(color),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                )
                .padding([5, 6])
                .width(Fill)
                .on_press_maybe(enabled.then(|| {
                    Message::AgentFollowUpSelected(AgentPrompt::from(follow_up.to_string()))
                }))
                .style(move |theme, status| agent_follow_up_style(theme, status, stagger)),
            );
    }
    rows.into()
}

fn agent_follow_ups(evidence: &[AgentEvidence<'_>]) -> Vec<&'static str> {
    let mut follow_ups = Vec::with_capacity(2);
    let used = |name: &str| evidence.iter().any(|item| item.name == name);
    if used("kerosene_journal") {
        follow_ups.push("Show the recurring pattern in my weakest trades");
    }
    if used("kerosene_risk") || used("kerosene_calculate") {
        follow_ups.push("Stress this conclusion with a 5% adverse move");
    }
    if follow_ups.len() < 2 && (used("kerosene_ohlcv") || used("kerosene_sessions")) {
        follow_ups.push("Compare this with the previous market session");
    }
    if follow_ups.len() < 2 && used("kerosene_market_data") {
        follow_ups.push("Compare this with the active market");
    }
    if follow_ups.len() < 2 {
        follow_ups.push("What evidence matters most here?");
    }
    if follow_ups.len() < 2 {
        follow_ups.push("Turn this into a concise risk checklist");
    }
    follow_ups.truncate(2);
    follow_ups
}

#[derive(Debug, Clone, Copy)]
struct AgentMarkdownViewer;

impl<'a> markdown::Viewer<'a, Message> for AgentMarkdownViewer {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::AgentOpenLink(url.into())
    }
}

fn agent_streaming_markdown<'a>(
    content: &'a markdown::Content,
    settings: markdown::Settings,
    base_color: Color,
    word_progress: f32,
    cursor_visible: bool,
) -> Element<'a, Message> {
    let viewer = AgentMarkdownViewer;
    let last_index = content.items().len().saturating_sub(1);
    let animate_paragraph = matches!(content.items().last(), Some(markdown::Item::Paragraph(_)));
    let mut blocks = Column::new().spacing(settings.spacing);
    for (index, item) in content.items().iter().enumerate() {
        if index == last_index
            && let markdown::Item::Paragraph(paragraph) = item
        {
            blocks = blocks.push(agent_streaming_paragraph(
                paragraph,
                settings,
                base_color,
                word_progress,
                cursor_visible,
            ));
        } else {
            blocks = blocks.push(markdown::item(&viewer, settings, item, index));
        }
    }
    if !animate_paragraph {
        let cursor_color = with_alpha(base_color, if cursor_visible { 0.95 } else { 0.18 });
        blocks = blocks.push(text("▋").size(settings.text_size).color(cursor_color));
    }
    blocks.into()
}

fn agent_streaming_paragraph<'a>(
    paragraph: &'a markdown::Text,
    settings: markdown::Settings,
    base_color: Color,
    word_progress: f32,
    cursor_visible: bool,
) -> Element<'a, Message> {
    let mut spans = paragraph.spans(settings.style).as_ref().to_vec();
    animate_latest_span(&mut spans, base_color, word_progress);
    let cursor_color = with_alpha(base_color, if cursor_visible { 0.95 } else { 0.18 });
    spans.push(iced::widget::text::Span::new("▋").color(cursor_color));
    rich_text(spans)
        .size(settings.text_size)
        .on_link_click(|url| Message::AgentOpenLink(url.into()))
        .into()
}

fn animate_latest_span(
    spans: &mut Vec<iced::widget::text::Span<'static, markdown::Uri>>,
    base_color: Color,
    progress: f32,
) {
    let Some(index) = spans.iter().rposition(|span| !span.text.trim().is_empty()) else {
        return;
    };
    let original = spans[index].clone();
    let raw = original.text.as_ref();
    let trimmed = raw.trim_end_matches(char::is_whitespace);
    if trimmed.is_empty() {
        return;
    }
    let word_start = trimmed
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            character
                .is_whitespace()
                .then_some(index + character.len_utf8())
        })
        .unwrap_or_default();
    let word_end = trimmed.len();
    let mut replacements = Vec::with_capacity(3);
    if word_start > 0 {
        let mut prefix = original.clone();
        prefix.text = raw[..word_start].to_string().into();
        replacements.push(prefix);
    }
    let eased = 1.0 - (1.0 - progress.clamp(0.0, 1.0)).powi(3);
    let mut animated = original.clone();
    animated.text = raw[word_start..word_end].to_string().into();
    let mut animated_color = animated.color.unwrap_or(base_color);
    animated_color.a *= 0.18 + eased * 0.82;
    animated.color = Some(animated_color);
    replacements.push(animated);
    if word_end < raw.len() {
        let mut suffix = original.clone();
        suffix.text = raw[word_end..].to_string().into();
        replacements.push(suffix);
    }
    spans.splice(index..=index, replacements);
}

fn with_alpha(mut color: Color, alpha: f32) -> Color {
    color.a *= alpha.clamp(0.0, 1.0);
    color
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

fn agent_response_action_style(
    theme: &Theme,
    status: button::Status,
    progress: f32,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = theme.palette().text;
    background.a = if hovered { 0.07 * progress } else { 0.0 };
    button::Style {
        background: Some(background.into()),
        text_color: with_alpha(theme.palette().text, progress),
        border: Border {
            radius: 5.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_follow_up_style(theme: &Theme, status: button::Status, progress: f32) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = theme.palette().primary;
    background.a = if hovered { 0.07 * progress } else { 0.0 };
    let mut border = theme.extended_palette().background.strong.color;
    border.a *= progress;
    button::Style {
        background: Some(background.into()),
        text_color: with_alpha(theme.palette().text, progress),
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: border,
        },
        ..Default::default()
    }
}

fn agent_evidence_style(theme: &Theme) -> container_style::Style {
    let mut background = theme.extended_palette().background.strong.color;
    background.a = 0.32;
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            radius: 7.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    }
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

fn agent_pnl_card_hover_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(with_alpha(theme.palette().primary, 0.10).into()),
        border: Border {
            color: with_alpha(theme.palette().primary, 0.8),
            width: 1.0,
            radius: 7.0.into(),
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

    #[test]
    fn follow_ups_are_derived_from_actual_tool_categories() {
        let evidence = vec![
            AgentEvidence {
                name: "kerosene_journal",
                detail: None,
                finished: true,
                is_error: false,
            },
            AgentEvidence {
                name: "kerosene_risk",
                detail: None,
                finished: true,
                is_error: false,
            },
        ];

        assert_eq!(
            agent_follow_ups(&evidence),
            vec![
                "Show the recurring pattern in my weakest trades",
                "Stress this conclusion with a 5% adverse move",
            ]
        );
    }

    #[test]
    fn evidence_is_scoped_to_the_response_turn() {
        let entries = vec![
            AgentChatEntry::Tool {
                call_id: "old".to_string(),
                name: "kerosene_data".to_string(),
                detail: None,
                finished: true,
                is_error: false,
            },
            AgentChatEntry::Message {
                role: AgentChatRole::User,
                text: "Question".to_string(),
                markdown: None,
            },
            AgentChatEntry::Tool {
                call_id: "current".to_string(),
                name: "kerosene_risk".to_string(),
                detail: Some("Current portfolio".to_string()),
                finished: true,
                is_error: false,
            },
            AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                text: "Answer".to_string(),
                markdown: Some(Box::new(markdown::Content::parse("Answer"))),
            },
        ];

        let evidence = agent_turn_evidence(&entries, 3);

        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].name, "kerosene_risk");
    }

    #[test]
    fn newest_stream_span_fades_without_changing_text() {
        let mut spans: Vec<iced::widget::text::Span<'static, markdown::Uri>> =
            vec![iced::widget::text::Span::new("Resolved newest ")];

        animate_latest_span(&mut spans, Color::WHITE, 0.0);

        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_ref())
                .collect::<String>(),
            "Resolved newest "
        );
        assert!(
            spans
                .iter()
                .any(|span| span.color.is_some_and(|color| color.a < 0.5))
        );
    }
}
