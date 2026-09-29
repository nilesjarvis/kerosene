use super::composer::compact_token_count;
use super::styles::{agent_empty_card_style, agent_model_option_style, agent_model_picker_style};
use crate::app_state::TradingTerminal;
use crate::config::AssistantProvider;
use crate::helpers;
use crate::message::Message;
use crate::openrouter_api::OpenRouterModel;
use iced::widget::{
    Column, Space, button, column, container, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Element, Fill, Length, Theme};

const MAX_VISIBLE_MODEL_RESULTS: usize = 80;

impl TradingTerminal {
    pub(super) fn view_agent_model_picker<'a>(
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
