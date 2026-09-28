use super::styles::{
    agent_composer_add_button_style, agent_composer_input_style, agent_composer_style,
    agent_empty_card_style, agent_pnl_card_hover_style, agent_prompt_action_button_style,
    agent_prompt_model_button_style, with_alpha,
};
use crate::agent_state::{AGENT_PRESENTATION_TICK_MS, AgentState, AgentStatus};
use crate::app_fonts;
use crate::app_state::TradingTerminal;
use crate::config::AssistantProvider;
use crate::helpers;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Row, Space, button, column, container, image, row, text, text_input};
use iced::{Alignment, Border, ContentFit, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(super) fn view_agent_composer(&self, theme: &Theme) -> Element<'_, Message> {
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
            agent_loading_activity(&self.agent, theme)
        } else {
            Space::new().height(Length::Fixed(0.0)).into()
        };

        let mut composer = Column::new()
            .push(status_detail)
            .push(configure)
            .push(self.view_agent_pnl_card_attachment(theme))
            .push(loading_activity)
            .spacing(7);
        if self.agent.model_picker_open {
            composer = composer.push(self.view_agent_model_picker(&requested_model, theme));
        }
        composer = composer.push(composer_bar).push(footer);

        composer.into()
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

pub(super) fn compact_token_count(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests;
