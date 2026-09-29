use super::streaming::{agent_markdown_settings, agent_streaming_markdown};
use super::styles::{
    agent_empty_card_style, agent_follow_up_style, agent_reasoning_button_style,
    agent_reasoning_rail_style, agent_response_action_style, agent_tool_trace_row_style,
    user_bubble_style, with_alpha,
};
use crate::agent_state::{
    AGENT_PRESENTATION_TICK_MS, AgentChatEntry, AgentChatRole, AgentPrompt, AgentState,
    agent_tool_presentation,
};
use crate::app_fonts;
use crate::message::Message;
use iced::widget::{Column, Space, button, container, markdown, row, text};
use iced::{Alignment, Element, Fill, Theme};

pub(super) fn agent_entry<'a>(
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
            follow_ups,
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
                let progress = agent.stream.completion_progress;
                content = content.push(agent_response_actions(
                    entry_index,
                    !agent.featured_response_has_image,
                    progress,
                    theme,
                ));
                if !follow_ups.is_empty() {
                    content = content.push(agent_follow_up_view(follow_ups, progress, theme));
                }
            }

            let bubble = container(content).padding([10, 12]).style(match role {
                AgentChatRole::User => user_bubble_style,
                AgentChatRole::Assistant => agent_empty_card_style,
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

pub(super) fn agent_tool_trace_starts_at(entries: &[AgentChatEntry], entry_index: usize) -> bool {
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
    follow_ups: &'a [String],
    progress: f32,
    theme: &Theme,
) -> Element<'a, Message> {
    let enabled = progress >= 0.72;
    let muted = with_alpha(
        theme.extended_palette().background.weak.text,
        0.2 + progress * 0.8,
    );
    let mut rows = Column::new()
        .push(text("Follow-ups").size(10).color(muted))
        .spacing(2)
        .width(Fill);
    for (index, follow_up) in follow_ups.iter().enumerate() {
        let stagger = ((progress - index as f32 * 0.18) / 0.82).clamp(0.0, 1.0);
        let color = with_alpha(theme.palette().text, 0.12 + stagger * 0.88);
        rows = rows.push(
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
            .on_press_maybe(
                enabled
                    .then(|| Message::AgentFollowUpSelected(AgentPrompt::from(follow_up.clone()))),
            )
            .style(move |theme, status| agent_follow_up_style(theme, status, stagger)),
        );
    }
    rows.into()
}

#[cfg(test)]
mod tests;
