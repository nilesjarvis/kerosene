use crate::agent_state::{
    AGENT_PRESENTATION_TICK_MS, AgentChatEntry, AgentChatRole, AgentPrompt, AgentState,
    AgentStatus, agent_tool_presentation,
};
use crate::app_fonts;
use crate::app_state::TradingTerminal;
use crate::config::AssistantProvider;
use crate::helpers;
use crate::message::Message;
use crate::openrouter_api::OpenRouterModel;

use iced::widget::container as container_style;
use iced::widget::{
    Column, Row, Space, button, column, container, image, markdown, rich_text, row, rule,
    scrollable, text, text_input,
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
        let requested_model = self
            .assistant_model_for_task()
            .unwrap_or_else(|| "No model detected".to_string());
        let image_model_ready =
            !has_pnl_card || self.assistant_model_supports_images(&requested_model) == Some(true);
        let can_send = self.assistant_configured()
            && !self.agent.status.is_busy()
            && !self.agent.pnl_card_loading
            && image_model_ready
            && (!self.agent.input.trim().is_empty() || has_pnl_card);
        let input = text_input(
            if has_pnl_card {
                "Add context…"
            } else {
                "Write a message…"
            },
            &self.agent.input,
        )
        .id(iced::widget::Id::new("kerosene-agent-input"))
        .style(agent_composer_input_style)
        .on_input(|value| Message::AgentInputChanged(value.into()))
        .on_submit_maybe(can_send.then_some(Message::AgentSubmit))
        .padding([9, 6])
        .size(13)
        .width(Fill);

        let action = if self.agent.status == AgentStatus::Thinking {
            button(text("■").size(10))
                .padding([7, 10])
                .on_press(Message::AgentAbort)
                .style(agent_prompt_action_button_style)
        } else {
            button(text("↑").size(17))
                .padding([4, 9])
                .on_press_maybe(can_send.then_some(Message::AgentSubmit))
                .style(agent_prompt_action_button_style)
        };

        let context_model_key = self.assistant_context_model_key(&requested_model);
        let (runtime_model, context_tokens, context_window) =
            self.agent.context_metrics_for_model(&context_model_key);
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
        let model_name = self
            .agent
            .model_catalog
            .iter()
            .find(|model| model.id == display_model)
            .map_or(display_model, |model| model.name.as_str());
        let model_label = helpers::ellipsized_text(model_name, 22);
        let model_button = button(
            row![
                text(model_label).size(11),
                text(model_picker_caret)
                    .size(8)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([6, 8])
        .on_press_maybe((!self.agent.status.is_busy()).then_some(Message::AgentToggleModelPicker))
        .style(agent_prompt_model_button_style);
        let footer = row![
            text("Read-only data access")
                .size(9)
                .color(theme.palette().success),
            Space::new().width(Fill),
            text(context_and_usage)
                .size(9)
                .color(theme.extended_palette().background.weak.text),
        ]
        .align_y(Alignment::Center);

        let configure: Element<'_, Message> = match self.assistant_provider {
            AssistantProvider::OpenRouter if !self.openrouter_configured() => {
                button(text("Configure OpenRouter").size(11))
                    .padding([7, 10])
                    .on_press(Message::OpenIntegrationsSettings)
                    .into()
            }
            AssistantProvider::LlamaCpp if !self.assistant_configured() => button(
                text(if self.agent.local_detection_loading {
                    "Detecting local llama.cpp…"
                } else {
                    "Refresh local detection"
                })
                .size(11),
            )
            .padding([7, 10])
            .on_press_maybe(
                (!self.agent.local_detection_loading).then_some(Message::AgentRefreshModels),
            )
            .into(),
            _ => Space::new().height(Length::Fixed(0.0)).into(),
        };

        let attach = button(text("+").size(18))
            .padding([5, 9])
            .on_press_maybe(
                (!self.agent.status.is_busy() && !self.agent.pnl_card_loading)
                    .then_some(Message::AgentPnlCardBrowse),
            )
            .style(agent_composer_add_button_style);
        let composer_bar = container(
            row![attach, input, model_button, action]
                .spacing(4)
                .align_y(Alignment::Center),
        )
        .padding(6)
        .width(Fill)
        .style(agent_composer_style);

        let loading_activity: Element<'_, Message> = if self.agent.status.is_busy() {
            agent_loading_activity(&self.agent, &theme)
        } else {
            Space::new().height(Length::Fixed(0.0)).into()
        };

        let mut composer = Column::new()
            .push(status_detail)
            .push(configure)
            .push(self.view_agent_pnl_card_attachment(&theme))
            .push(loading_activity)
            .spacing(7);
        if self.agent.model_picker_open {
            composer = composer.push(self.view_agent_model_picker(&requested_model, &theme));
        }
        composer = composer.push(composer_bar).push(footer);

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
        let (persistence_label, persistence_color) =
            if let Some(error) = &self.agent.persistence_error {
                (helpers::ellipsized_text(error, 28), theme.palette().danger)
            } else if self.agent.persistence_in_flight || self.agent.persistence_dirty {
                (
                    "Saving locally…".to_string(),
                    theme.extended_palette().background.weak.text,
                )
            } else {
                ("Saved locally".to_string(), theme.palette().success)
            };

        if self.agent.sidebar_collapsed {
            let expand_icon = container(text("›").size(20)).center(Length::Fixed(32.0));
            let expand = button(expand_icon)
                .padding(0)
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .on_press(Message::AgentToggleSidebar)
                .style(agent_sidebar_control_button_style);
            let new_session_icon = container(text("+").size(18)).center(Length::Fixed(32.0));
            let new_session = button(new_session_icon)
                .padding(0)
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .on_press_maybe(can_change_session.then_some(Message::AgentNewChat))
                .style(agent_sidebar_control_button_style);

            return container(
                column![
                    new_session,
                    Space::new().height(Fill),
                    text("●").size(7).color(persistence_color),
                    expand,
                ]
                .spacing(6)
                .align_x(Alignment::Center)
                .height(Fill),
            )
            .width(Length::Fixed(52.0))
            .height(Fill)
            .padding([12, 8])
            .style(agent_session_sidebar_style)
            .into();
        }

        let collapse = button(text("‹").size(20))
            .padding([4, 9])
            .on_press(Message::AgentToggleSidebar)
            .style(agent_sidebar_control_button_style);
        let new_session = button(
            row![text("+").size(17), text("New chat").size(13)]
                .spacing(9)
                .align_y(Alignment::Center),
        )
        .padding([7, 8])
        .width(Fill)
        .on_press_maybe(can_change_session.then_some(Message::AgentNewChat))
        .style(agent_sidebar_control_button_style);

        let mut sessions = Column::new().spacing(1).width(Fill);
        for item in self.agent.session_items() {
            let count = if item.message_count == 0 {
                String::new()
            } else {
                item.message_count.to_string()
            };
            let active = item.active;
            let title_color = if active {
                theme.palette().text
            } else {
                with_alpha(theme.palette().text, 0.78)
            };
            let content = row![
                text(helpers::ellipsized_text(item.title, 25))
                    .size(12)
                    .color(title_color)
                    .width(Fill),
                text(count)
                    .size(9)
                    .font(app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Fill);
            sessions = sessions.push(
                button(content)
                    .padding([7, 8])
                    .width(Fill)
                    .on_press_maybe(
                        (can_change_session && !active)
                            .then_some(Message::AgentSelectSession(item.id)),
                    )
                    .style(move |theme, status| agent_session_button_style(theme, status, active)),
            );
        }

        let sidebar_footer = row![
            text("●").size(7).color(persistence_color),
            text(persistence_label).size(9).color(persistence_color),
            Space::new().width(Fill),
            collapse,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        container(
            column![
                new_session,
                scrollable(sessions).height(Fill),
                sidebar_footer,
            ]
            .spacing(7)
            .height(Fill),
        )
        .width(Length::Fixed(224.0))
        .height(Fill)
        .padding([12, 8])
        .style(agent_session_sidebar_style)
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
        if self.assistant_provider == AssistantProvider::LlamaCpp {
            return self.view_agent_local_provider_picker(theme);
        }

        let provider_selector = self.view_agent_provider_selector(theme);
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
            .push(provider_selector)
            .push(rule::horizontal(1))
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

    fn view_agent_provider_selector(&self, theme: &Theme) -> Element<'_, Message> {
        let openrouter_selected = self.assistant_provider == AssistantProvider::OpenRouter;
        let local_selected = self.assistant_provider == AssistantProvider::LlamaCpp;
        let local_ready = self
            .agent
            .local_server
            .as_ref()
            .is_some_and(|server| server.supports_tools && server.primary_model().is_some());
        let local_label = if self.agent.local_detection_loading {
            "Local llama.cpp · detecting…".to_string()
        } else if let Some(server) = self.agent.local_server.as_ref() {
            if local_ready {
                format!("Local llama.cpp · {}", server.endpoint_label())
            } else {
                "Local llama.cpp · tools unavailable".to_string()
            }
        } else {
            "Local llama.cpp · not detected".to_string()
        };
        let can_change = !self.agent.status.is_busy();

        column![
            text("Assistant provider")
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            row![
                button(text("OpenRouter").size(10))
                    .padding([6, 10])
                    .on_press_maybe(
                        (can_change && !openrouter_selected).then_some(
                            Message::AgentProviderChanged(AssistantProvider::OpenRouter)
                        )
                    )
                    .style(move |theme, status| {
                        agent_model_option_style(theme, status, openrouter_selected)
                    }),
                button(text(local_label).size(10))
                    .padding([6, 10])
                    .on_press_maybe(
                        (can_change && local_ready && !local_selected)
                            .then_some(Message::AgentProviderChanged(AssistantProvider::LlamaCpp))
                    )
                    .style(move |theme, status| {
                        agent_model_option_style(theme, status, local_selected)
                    }),
            ]
            .spacing(6),
        ]
        .spacing(4)
        .into()
    }

    fn view_agent_local_provider_picker(&self, theme: &Theme) -> Element<'_, Message> {
        let provider_selector = self.view_agent_provider_selector(theme);
        let refresh = button(text("Refresh detection").size(10))
            .padding([5, 9])
            .on_press_maybe(
                (!self.agent.local_detection_loading).then_some(Message::AgentRefreshModels),
            );
        let close = button(text("×").size(13))
            .padding([4, 8])
            .on_press(Message::AgentToggleModelPicker);
        let header = row![
            column![
                text("Local llama.cpp provider")
                    .size(12)
                    .color(theme.palette().text),
                text("Auto-detected and verified over a loopback OpenAI-compatible API")
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

        let body: Element<'_, Message> = if self.agent.local_detection_loading {
            container(
                text(
                    "Looking for running llama-server processes and verifying their model catalog…",
                )
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            )
            .center_x(Fill)
            .padding(18)
            .into()
        } else if let Some(server) = self.agent.local_server.as_ref() {
            let model = server
                .primary_model()
                .map(|model| model.id.as_str())
                .unwrap_or("No model advertised");
            let context = server
                .primary_model()
                .and_then(|model| model.context_window)
                .map(compact_token_count)
                .unwrap_or_else(|| "Unknown context".to_string());
            let compatibility = format!(
                "{} · {} · {} context",
                if server.supports_tools {
                    "Tools"
                } else {
                    "No tool calling"
                },
                if server.supports_vision {
                    "Vision"
                } else {
                    "Text"
                },
                context
            );
            container(
                column![
                    row![
                        text(model).size(11).color(theme.palette().text).width(Fill),
                        text(if server.supports_tools {
                            "SELECTED"
                        } else {
                            "INCOMPATIBLE"
                        })
                        .size(8)
                        .color(if server.supports_tools {
                            theme.palette().primary
                        } else {
                            theme.palette().danger
                        }),
                    ]
                    .align_y(Alignment::Center),
                    text(format!("Loopback · {}", server.endpoint_label()))
                        .size(9)
                        .color(theme.palette().primary),
                    text(compatibility)
                        .size(9)
                        .color(theme.extended_palette().background.weak.text),
                    text("Local inference · no model API charge")
                        .size(9)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(3),
            )
            .padding([9, 10])
            .width(Fill)
            .style(agent_empty_card_style)
            .into()
        } else {
            let message = self.agent.local_detection_error.as_deref().unwrap_or(
                "No llama.cpp server detected. Start llama-server on this machine, then refresh detection.",
            );
            container(text(message).size(10).color(theme.palette().danger))
                .center_x(Fill)
                .padding(18)
                .into()
        };

        container(
            column![
                provider_selector,
                rule::horizontal(1),
                header,
                body,
                text("Only a verified loopback endpoint is accepted; model prompts do not leave this machine.")
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(7),
        )
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

fn agent_loading_activity(agent: &AgentState, theme: &Theme) -> Element<'static, Message> {
    let phase = agent.stream.cursor_phase;
    let mut grid = Column::new().spacing(2);
    for row_index in 0_i32..3 {
        let mut cells = Row::new().spacing(2);
        for column_index in 0_i32..3 {
            let delay_steps = column_index + (row_index - 1).abs();
            let delay = delay_steps as f32 * 90.0 / 650.0;
            let color = with_alpha(theme.palette().primary, loading_pixel_alpha(phase, delay));
            cells = cells.push(
                container(Space::new())
                    .width(Length::Fixed(4.0))
                    .height(Length::Fixed(4.0))
                    .style(move |_theme: &Theme| container_style::Style {
                        background: Some(color.into()),
                        border: Border {
                            radius: 1.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            );
        }
        grid = grid.push(cells);
    }

    let shimmer = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
    let label_color = with_alpha(theme.palette().text, 0.62 + shimmer * 0.32);
    let elapsed = format_activity_elapsed(
        agent
            .stream
            .activity_ticks
            .saturating_mul(AGENT_PRESENTATION_TICK_MS),
    );

    container(
        row![
            grid,
            text(agent.status.label()).size(11).color(label_color),
            text(elapsed)
                .size(10)
                .font(app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(9)
        .align_y(Alignment::Center),
    )
    .padding([2, 5])
    .into()
}

fn loading_pixel_alpha(phase: f32, delay: f32) -> f32 {
    let local = (phase - delay).rem_euclid(1.0);
    if local < 0.18 {
        0.15 + 0.85 * smoothstep(local / 0.18)
    } else if local < 0.42 {
        1.0
    } else if local < 0.62 {
        1.0 - 0.85 * smoothstep((local - 0.42) / 0.20)
    } else {
        0.15
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn format_activity_elapsed(elapsed_ms: u64) -> String {
    let elapsed_seconds = elapsed_ms as f64 / 1_000.0;
    if elapsed_seconds < 60.0 {
        format!("{elapsed_seconds:.1}s")
    } else {
        let minutes = (elapsed_seconds / 60.0).floor() as u64;
        format!("{minutes}m {:.1}s", elapsed_seconds % 60.0)
    }
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
                    !agent.featured_response_has_image,
                    progress,
                    theme,
                ));
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
        AgentChatEntry::Tool { expanded, .. } => {
            let tools = agent_tool_trace_items(&agent.entries, entry_index);
            agent_tool_trace(entry_index, &tools, *expanded, theme)
        }
        AgentChatEntry::Reasoning {
            text,
            elapsed_ticks,
            finished,
            expanded,
        } => agent_reasoning_trace(
            entry_index,
            text,
            *elapsed_ticks,
            *finished,
            *expanded,
            theme,
        ),
    }
}

fn agent_reasoning_trace<'a>(
    entry_index: usize,
    reasoning: &'a str,
    elapsed_ticks: u64,
    finished: bool,
    expanded: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let caret = if expanded { "⌃" } else { "⌄" };
    let header = button(
        row![
            text("✦").size(12).color(theme.palette().primary),
            text(reasoning_duration_label(elapsed_ticks, finished))
                .size(11)
                .color(muted),
            text(caret).size(11).color(muted),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([4, 2])
    .on_press(Message::AgentToggleReasoning(entry_index))
    .style(agent_reasoning_button_style);

    let mut trace = Column::new().push(header).spacing(2).width(Fill);
    if expanded && !reasoning.is_empty() {
        let rail = container(Space::new().width(1).height(Fill))
            .width(1)
            .height(Fill)
            .style(agent_reasoning_rail_style);
        let body = text(reasoning).size(12).color(muted).width(Fill);
        trace = trace
            .push(row![Space::new().width(9), rail, container(body).padding([3, 0]),].spacing(8));
    }

    container(trace)
        .padding([0, 12])
        .width(Fill)
        .max_width(660.0)
        .into()
}

fn reasoning_duration_label(elapsed_ticks: u64, finished: bool) -> String {
    let elapsed_ms = elapsed_ticks.saturating_mul(AGENT_PRESENTATION_TICK_MS);
    if !finished {
        return if elapsed_ms < 1_000 {
            "Thinking…".to_string()
        } else {
            format_reasoning_duration("Thinking for", elapsed_ms)
        };
    }
    if elapsed_ms < 1_000 {
        "Thought for less than a second".to_string()
    } else {
        format_reasoning_duration("Thought for", elapsed_ms)
    }
}

fn format_reasoning_duration(prefix: &str, elapsed_ms: u64) -> String {
    let seconds = elapsed_ms.saturating_add(500) / 1_000;
    let unit = if seconds == 1 { "second" } else { "seconds" };
    format!("{prefix} {seconds} {unit}")
}

struct AgentEvidence<'a> {
    name: &'a str,
    detail: Option<&'a str>,
    finished: bool,
    is_error: bool,
}

fn agent_tool_trace_starts_at(entries: &[AgentChatEntry], entry_index: usize) -> bool {
    let Some(AgentChatEntry::Tool { .. }) = entries.get(entry_index) else {
        return false;
    };
    let turn_start = entries[..entry_index]
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
    !entries[turn_start..entry_index]
        .iter()
        .any(|entry| matches!(entry, AgentChatEntry::Tool { .. }))
}

fn agent_tool_trace_items(
    entries: &[AgentChatEntry],
    entry_index: usize,
) -> Vec<AgentEvidence<'_>> {
    let turn_end = entries[entry_index..]
        .iter()
        .position(|entry| {
            matches!(
                entry,
                AgentChatEntry::Message {
                    role: AgentChatRole::User,
                    ..
                }
            )
        })
        .map_or(entries.len(), |offset| entry_index + offset);
    entries[entry_index..turn_end]
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
            AgentChatEntry::Message { .. } | AgentChatEntry::Reasoning { .. } => None,
        })
        .collect()
}

fn agent_tool_trace<'a>(
    entry_index: usize,
    tools: &[AgentEvidence<'a>],
    expanded: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let running = tools.iter().filter(|tool| !tool.finished).count();
    let failed = tools.iter().filter(|tool| tool.is_error).count();
    let count = tools.len();
    let noun = if count == 1 { "tool" } else { "tools" };
    let label = if running > 0 {
        format!("Running {count} {noun}")
    } else {
        format!("Ran {count} {noun}")
    };
    let caret = if expanded { "⌃" } else { "⌄" };
    let mut header_content = row![
        text("✦").size(12).color(theme.palette().primary),
        text(label).size(11).color(muted),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    if failed > 0 {
        header_content = header_content.push(
            text(format!("{failed} failed"))
                .size(10)
                .color(theme.palette().danger),
        );
    }
    header_content = header_content.push(text(caret).size(11).color(muted));
    let header = button(header_content)
        .padding([4, 2])
        .on_press(Message::AgentToggleToolTrace(entry_index))
        .style(agent_reasoning_button_style);

    let mut trace = Column::new().push(header).spacing(2).width(Fill);
    if expanded && !tools.is_empty() {
        let rail = container(Space::new().width(1).height(Fill))
            .width(1)
            .height(Fill)
            .style(agent_reasoning_rail_style);
        let mut rows = Column::new().spacing(2).width(Fill);
        for tool in tools {
            let presentation = agent_tool_presentation(tool.name);
            let detail = tool.detail.unwrap_or(presentation.title);
            let mut row_content = row![
                text(agent_tool_trace_action(tool.name))
                    .size(12)
                    .color(theme.palette().text),
                text(detail)
                    .size(11)
                    .font(app_fonts::monospace_font())
                    .color(muted)
                    .width(Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            let state_color = if tool.is_error {
                Some(("Failed", theme.palette().danger))
            } else if !tool.finished {
                Some(("Running", theme.palette().warning))
            } else {
                None
            };
            if let Some((state, color)) = state_color {
                row_content = row_content.push(text(state).size(10).color(color));
            }
            let row_color = state_color.map(|(_state, color)| color);
            rows = rows.push(
                container(row_content)
                    .padding([5, 6])
                    .width(Fill)
                    .style(move |theme: &Theme| agent_tool_trace_row_style(theme, row_color)),
            );
        }
        trace = trace
            .push(row![Space::new().width(9), rail, container(rows).padding([3, 0]),].spacing(8));
    }

    container(trace)
        .padding([0, 12])
        .width(Fill)
        .max_width(660.0)
        .into()
}

fn agent_tool_trace_action(name: &str) -> &'static str {
    match name {
        "kerosene_data" | "kerosene_activity" | "kerosene_journal" => "Read",
        "kerosene_market_data" | "kerosene_positioning" | "kerosene_ohlcv" => "Fetch",
        "kerosene_calculate" | "kerosene_sessions" => "Calculate",
        "kerosene_risk" => "Analyze",
        "kerosene_pnl_card_match" => "Match",
        _ => "Run",
    }
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
            AgentChatEntry::Message { .. } | AgentChatEntry::Reasoning { .. } => None,
        })
        .collect()
}

fn agent_response_actions<'a>(
    entry_index: usize,
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

    row![copy, retry]
        .spacing(2)
        .align_y(Alignment::Center)
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

fn agent_reasoning_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut background = theme.palette().text;
    background.a = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        0.045
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.extended_palette().background.weak.text,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_reasoning_rail_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        ..Default::default()
    }
}

fn agent_tool_trace_row_style(
    _theme: &Theme,
    state_color: Option<Color>,
) -> container_style::Style {
    container_style::Style {
        background: state_color.map(|mut color| {
            color.a = 0.045;
            color.into()
        }),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_session_sidebar_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..Default::default()
    }
}

fn agent_sidebar_control_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut background = theme.palette().text;
    background.a = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        0.06
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: if matches!(status, button::Status::Disabled) {
            theme.extended_palette().background.weak.text
        } else {
            with_alpha(theme.palette().text, 0.82)
        },
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_session_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut background = theme.palette().text;
    background.a = if active {
        0.075
    } else if hovered {
        0.05
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_prompt_model_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut background = theme.palette().primary;
    background.a = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        0.09
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_composer_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = helpers::text_input_style(theme, status);
    style.background = Color::TRANSPARENT.into();
    style.border = Border::default();
    style
}

fn agent_composer_style(theme: &Theme) -> container_style::Style {
    let mut shadow_color = Color::BLACK;
    shadow_color.a = 0.16;
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        shadow: iced::Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

fn agent_composer_add_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let disabled = matches!(status, button::Status::Disabled);
    let mut background = theme.palette().text;
    background.a = if disabled {
        0.0
    } else if hovered {
        0.08
    } else {
        0.0
    };
    button::Style {
        background: Some(background.into()),
        text_color: if disabled {
            theme.extended_palette().background.weak.text
        } else {
            theme.palette().text
        },
        border: Border {
            radius: 7.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_prompt_action_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    let pressed = matches!(status, button::Status::Pressed);
    let mut background = if disabled {
        theme.extended_palette().background.strong.color
    } else {
        theme.palette().primary
    };
    if pressed {
        background.a *= 0.82;
    }
    button::Style {
        background: Some(background.into()),
        text_color: if disabled {
            theme.extended_palette().background.weak.text
        } else {
            theme.extended_palette().primary.base.text
        },
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn agent_model_picker_style(theme: &Theme) -> container_style::Style {
    let mut shadow_color = Color::BLACK;
    shadow_color.a = 0.20;
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        shadow: iced::Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
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
    fn loading_activity_formats_short_and_long_elapsed_times() {
        assert_eq!(format_activity_elapsed(3_456), "3.5s");
        assert_eq!(format_activity_elapsed(62_340), "1m 2.3s");
    }

    #[test]
    fn loading_drive_wave_keeps_pixels_dim_between_fronts() {
        assert_eq!(loading_pixel_alpha(0.8, 0.0), 0.15);
        assert!(loading_pixel_alpha(0.3, 0.0) > 0.95);
        assert!((0.15..=1.0).contains(&loading_pixel_alpha(0.12, 0.3)));
    }

    #[test]
    fn reasoning_duration_matches_thinking_disclosure_copy() {
        assert_eq!(reasoning_duration_label(0, false), "Thinking…");
        assert_eq!(reasoning_duration_label(63, true), "Thought for 1 second");
        assert_eq!(reasoning_duration_label(250, true), "Thought for 4 seconds");
    }

    #[test]
    fn tool_trace_groups_calls_across_reasoning_within_one_turn() {
        let entries = vec![
            AgentChatEntry::Message {
                role: AgentChatRole::User,
                text: "Inspect my risk".to_string(),
                markdown: None,
            },
            AgentChatEntry::Tool {
                call_id: "positions".to_string(),
                name: "kerosene_data".to_string(),
                detail: Some("Open positions".to_string()),
                finished: true,
                is_error: false,
                expanded: true,
            },
            AgentChatEntry::Reasoning {
                text: "Checking concentration".to_string(),
                elapsed_ticks: 1,
                finished: true,
                expanded: true,
            },
            AgentChatEntry::Tool {
                call_id: "risk".to_string(),
                name: "kerosene_risk".to_string(),
                detail: Some("5% adverse move".to_string()),
                finished: false,
                is_error: false,
                expanded: true,
            },
            AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                text: "Answer".to_string(),
                markdown: None,
            },
            AgentChatEntry::Message {
                role: AgentChatRole::User,
                text: "Next question".to_string(),
                markdown: None,
            },
        ];

        assert!(agent_tool_trace_starts_at(&entries, 1));
        assert!(!agent_tool_trace_starts_at(&entries, 3));
        let tools = agent_tool_trace_items(&entries, 1);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "kerosene_data");
        assert_eq!(tools[1].name, "kerosene_risk");
        assert!(!tools[1].finished);
    }

    #[test]
    fn tool_trace_uses_short_coding_style_action_labels() {
        assert_eq!(agent_tool_trace_action("kerosene_data"), "Read");
        assert_eq!(agent_tool_trace_action("kerosene_market_data"), "Fetch");
        assert_eq!(agent_tool_trace_action("kerosene_calculate"), "Calculate");
        assert_eq!(agent_tool_trace_action("kerosene_risk"), "Analyze");
        assert_eq!(agent_tool_trace_action("unknown_tool"), "Run");
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
                expanded: true,
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
                expanded: true,
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
