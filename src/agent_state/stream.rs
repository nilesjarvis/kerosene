use super::{AgentChatEntry, AgentChatRole, AgentState, AgentToolPresentation, bounded_text};
use iced::widget::markdown;

const MAX_REASONING_BYTES: usize = 100_000;
const MAX_FOLLOW_UPS: usize = 2;
const MAX_FOLLOW_UP_CHARS: usize = 180;
const FOLLOW_UP_SECTION_START: &str = "<!-- KEROSENE_FOLLOW_UPS_V1";
const FOLLOW_UP_SECTION_END: &str = "KEROSENE_FOLLOW_UPS_V1 -->";

const STREAM_REVEAL_FRAME_INTERVAL: u8 = 3;
const STREAM_WORD_FADE_STEP: f32 = 0.14;
const STREAM_COMPLETION_FADE_STEP: f32 = 0.08;
const STREAM_CURSOR_PHASE_STEP: f32 = 0.025;
pub(crate) const AGENT_PRESENTATION_TICK_MS: u64 = 16;

pub(crate) struct AgentStreamPresentation {
    pending: String,
    transport_settled: bool,
    reveal_frame: u8,
    pub(crate) word_progress: f32,
    pub(crate) cursor_visible: bool,
    pub(crate) cursor_phase: f32,
    pub(crate) activity_ticks: u64,
    pub(crate) featured_entry_index: Option<usize>,
    pub(crate) completion_progress: f32,
}

impl Default for AgentStreamPresentation {
    fn default() -> Self {
        Self {
            pending: String::new(),
            transport_settled: false,
            reveal_frame: STREAM_REVEAL_FRAME_INTERVAL,
            word_progress: 1.0,
            cursor_visible: true,
            cursor_phase: 0.0,
            activity_ticks: 0,
            featured_entry_index: None,
            completion_progress: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Assistant response, reasoning, and tool presentation
// ---------------------------------------------------------------------------

pub(crate) fn agent_tool_presentation(name: &str) -> AgentToolPresentation {
    match name {
        "kerosene_data" => AgentToolPresentation {
            category: "Snapshot",
            title: "Current Kerosene snapshot",
            running_label: "Reading current Kerosene snapshot",
        },
        "kerosene_market_data" => AgentToolPresentation {
            category: "Markets",
            title: "Market lookup",
            running_label: "Looking up current market data",
        },
        "kerosene_set_chart_indicators" => AgentToolPresentation {
            category: "Workspace",
            title: "Chart indicators",
            running_label: "Updating chart indicators",
        },
        "kerosene_manage_chart_drawings" => AgentToolPresentation {
            category: "Workspace",
            title: "Chart drawings",
            running_label: "Updating chart drawings",
        },
        "kerosene_activity" => AgentToolPresentation {
            category: "Activity",
            title: "Account activity",
            running_label: "Reviewing account activity",
        },
        "kerosene_journal" => AgentToolPresentation {
            category: "Journal",
            title: "Trading journal",
            running_label: "Reviewing the trading journal",
        },
        "kerosene_calculate" => AgentToolPresentation {
            category: "Analysis",
            title: "Deterministic analysis",
            running_label: "Running deterministic analysis",
        },
        "kerosene_risk" => AgentToolPresentation {
            category: "Risk",
            title: "Portfolio-margin risk",
            running_label: "Reviewing portfolio-margin risk",
        },
        "kerosene_positioning" => AgentToolPresentation {
            category: "Positioning",
            title: "Aggregate positioning",
            running_label: "Fetching aggregate positioning",
        },
        "kerosene_pnl_card_match" => AgentToolPresentation {
            category: "P&L card",
            title: "Public position candidates",
            running_label: "Matching the card against public positions",
        },
        "kerosene_ohlcv" => AgentToolPresentation {
            category: "Price data",
            title: "Price history",
            running_label: "Fetching price history",
        },
        "kerosene_sessions" => AgentToolPresentation {
            category: "Sessions",
            title: "Market-session statistics",
            running_label: "Calculating market-session statistics",
        },
        _ => AgentToolPresentation {
            category: "Data",
            title: "Kerosene data access",
            running_label: "Reading Kerosene data",
        },
    }
}

impl AgentState {
    pub(crate) fn append_assistant_delta(&mut self, delta: &str) {
        self.finish_reasoning();
        if !delta.trim().is_empty() {
            self.current_turn_has_text = true;
        }
        let entry_index = self.assistant_entry_index.unwrap_or_else(|| {
            self.entries.push(AgentChatEntry::Message {
                role: AgentChatRole::Assistant,
                text: String::new(),
                markdown: Some(Box::new(markdown::Content::new())),
                follow_ups: Vec::new(),
            });
            let index = self.entries.len().saturating_sub(1);
            self.assistant_entry_index = Some(index);
            index
        });

        if let Some(AgentChatEntry::Message { text, markdown, .. }) =
            self.entries.get_mut(entry_index)
        {
            text.push_str(delta);
            if markdown.is_none() {
                *markdown = Some(Box::new(markdown::Content::new()));
            }
        }
        self.stream.pending.push_str(delta);
        self.stream.transport_settled = false;
        self.stream.reveal_frame = STREAM_REVEAL_FRAME_INTERVAL;
        self.stream.cursor_visible = true;
    }

    pub(crate) fn begin_reasoning(&mut self) {
        if self.reasoning_entry_index.is_some() {
            return;
        }
        self.entries.push(AgentChatEntry::Reasoning {
            text: String::new(),
            elapsed_ticks: 0,
            finished: false,
            expanded: true,
        });
        self.reasoning_entry_index = Some(self.entries.len().saturating_sub(1));
    }

    pub(crate) fn append_reasoning_delta(&mut self, delta: &str) {
        self.begin_reasoning();
        let Some(AgentChatEntry::Reasoning { text, .. }) = self
            .reasoning_entry_index
            .and_then(|index| self.entries.get_mut(index))
        else {
            return;
        };
        let remaining = MAX_REASONING_BYTES.saturating_sub(text.len());
        let mut prefix_len = remaining.min(delta.len());
        while !delta.is_char_boundary(prefix_len) {
            prefix_len = prefix_len.saturating_sub(1);
        }
        text.push_str(&delta[..prefix_len]);
    }

    pub(crate) fn finish_reasoning(&mut self) {
        let Some(index) = self.reasoning_entry_index.take() else {
            return;
        };
        if let Some(AgentChatEntry::Reasoning { finished, .. }) = self.entries.get_mut(index) {
            *finished = true;
        }
    }

    pub(crate) fn toggle_reasoning(&mut self, entry_index: usize) {
        if let Some(AgentChatEntry::Reasoning { expanded, .. }) = self.entries.get_mut(entry_index)
        {
            *expanded = !*expanded;
        }
    }

    pub(crate) fn mark_assistant_transport_settled(&mut self) {
        self.stream.transport_settled = true;
        self.stream.reveal_frame = STREAM_REVEAL_FRAME_INTERVAL;
    }

    pub(crate) fn finalize_assistant_response_metadata(&mut self) -> bool {
        let pending_len = self.stream.pending.len();
        let Some(entry_index) = self
            .assistant_entry_index
            .or_else(|| latest_assistant_after_last_user(&self.entries))
        else {
            self.current_turn_has_text = false;
            return false;
        };
        let Some(AgentChatEntry::Message {
            role: AgentChatRole::Assistant,
            text,
            markdown,
            follow_ups,
        }) = self.entries.get_mut(entry_index)
        else {
            self.current_turn_has_text = false;
            return false;
        };

        let visible_len = text.len().saturating_sub(pending_len);
        let (visible_answer, parsed_follow_ups) = split_assistant_follow_ups(text);
        if visible_answer != *text {
            *text = visible_answer;
            if visible_len <= text.len() && text.is_char_boundary(visible_len) {
                self.stream.pending = text[visible_len..].to_string();
            } else {
                self.stream.pending.clear();
                *markdown = Some(Box::new(markdown::Content::parse(text)));
            }
        }
        if let Some(parsed_follow_ups) = parsed_follow_ups {
            *follow_ups = parsed_follow_ups;
        }
        self.current_turn_has_text = !text.trim().is_empty();
        self.current_turn_has_text
    }

    pub(crate) fn assistant_stream_ready_to_finalize(&self) -> bool {
        self.stream.transport_settled && self.stream.pending.is_empty()
    }

    pub(crate) fn advance_assistant_stream(&mut self) -> (bool, bool) {
        self.stream.word_progress = (self.stream.word_progress + STREAM_WORD_FADE_STEP).min(1.0);
        self.stream.cursor_phase = (self.stream.cursor_phase + STREAM_CURSOR_PHASE_STEP).fract();
        self.stream.cursor_visible = self.stream.cursor_phase < 0.58;
        if self.status.is_busy() {
            self.stream.activity_ticks = self.stream.activity_ticks.saturating_add(1);
        }
        if let Some(AgentChatEntry::Reasoning {
            elapsed_ticks,
            finished: false,
            ..
        }) = self
            .reasoning_entry_index
            .and_then(|index| self.entries.get_mut(index))
        {
            *elapsed_ticks = elapsed_ticks.saturating_add(1);
        }
        if self.stream.featured_entry_index.is_some() {
            self.stream.completion_progress =
                (self.stream.completion_progress + STREAM_COMPLETION_FADE_STEP).min(1.0);
        }

        let mut visible_changed = false;
        if !self.stream.pending.is_empty() {
            self.stream.reveal_frame = self.stream.reveal_frame.saturating_add(1);
            if self.stream.reveal_frame >= STREAM_REVEAL_FRAME_INTERVAL {
                let units = if self.stream.transport_settled {
                    usize::MAX
                } else {
                    reveal_units_for_backlog(self.stream.pending.len())
                };
                let prefix_len =
                    reveal_prefix_len(&self.stream.pending, units, self.stream.transport_settled);
                if prefix_len > 0 {
                    let remainder = self.stream.pending.split_off(prefix_len);
                    let visible = std::mem::replace(&mut self.stream.pending, remainder);
                    self.append_visible_assistant_text(&visible);
                    self.stream.reveal_frame = 0;
                    self.stream.word_progress = 0.0;
                    visible_changed = true;
                }
            }
        }

        (
            visible_changed,
            self.stream.transport_settled && self.stream.pending.is_empty(),
        )
    }

    pub(crate) fn flush_assistant_stream(&mut self) -> bool {
        let visible = std::mem::take(&mut self.stream.pending);
        if visible.is_empty() {
            return false;
        }
        self.append_visible_assistant_text(&visible);
        self.stream.word_progress = 1.0;
        true
    }

    pub(crate) fn finish_assistant_presentation(&mut self) -> Option<usize> {
        self.finalize_assistant_response_metadata();
        self.flush_assistant_stream();
        let featured = self
            .assistant_entry_index
            .or_else(|| latest_assistant_after_last_user(&self.entries));
        self.assistant_entry_index = None;
        self.reset_stream_activity();
        self.stream.featured_entry_index = featured;
        self.featured_response_has_image = self.current_turn_has_image && featured.is_some();
        self.current_turn_has_image = false;
        self.stream.completion_progress = if featured.is_some() { 0.0 } else { 1.0 };
        featured
    }

    pub(crate) fn feature_latest_assistant_immediately(&mut self) {
        self.finalize_assistant_response_metadata();
        self.flush_assistant_stream();
        self.assistant_entry_index = None;
        self.reset_stream_activity();
        self.stream.featured_entry_index = latest_assistant_after_last_user(&self.entries);
        self.featured_response_has_image =
            self.current_turn_has_image && self.stream.featured_entry_index.is_some();
        self.current_turn_has_image = false;
        self.stream.completion_progress = 1.0;
    }

    pub(crate) fn refresh_featured_assistant(&mut self) {
        self.stream.featured_entry_index = self.entries.iter().rposition(|entry| {
            matches!(
                entry,
                AgentChatEntry::Message {
                    role: AgentChatRole::Assistant,
                    ..
                }
            )
        });
        self.featured_response_has_image = false;
    }

    pub(crate) fn stream_needs_tick(&self) -> bool {
        let active_stream = self.assistant_entry_index.is_some()
            && (!self.stream.transport_settled
                || !self.stream.pending.is_empty()
                || self.stream.word_progress < 1.0);
        self.status.is_busy()
            || active_stream
            || (self.stream.featured_entry_index.is_some() && self.stream.completion_progress < 1.0)
    }

    pub(crate) fn toggle_tool_trace(&mut self, entry_index: usize) {
        if let Some(AgentChatEntry::Tool { expanded, .. }) = self.entries.get_mut(entry_index) {
            *expanded = !*expanded;
        }
    }

    pub(crate) fn reset_stream_for_entries_change(&mut self) {
        self.flush_assistant_stream();
        self.assistant_entry_index = None;
        self.reset_stream_activity();
        self.refresh_featured_assistant();
        self.featured_response_has_image = false;
        self.stream.completion_progress = 1.0;
    }

    fn append_visible_assistant_text(&mut self, visible: &str) {
        let Some(entry_index) = self.assistant_entry_index else {
            return;
        };
        if let Some(AgentChatEntry::Message { markdown, text, .. }) =
            self.entries.get_mut(entry_index)
        {
            if let Some(markdown) = markdown {
                markdown.push_str(visible);
            } else {
                let visible_len = text.len().saturating_sub(self.stream.pending.len());
                *markdown = Some(Box::new(markdown::Content::parse(&text[..visible_len])));
            }
        }
    }

    pub(super) fn reset_stream_activity(&mut self) {
        self.finish_reasoning();
        self.stream.pending.clear();
        self.stream.transport_settled = false;
        self.stream.reveal_frame = STREAM_REVEAL_FRAME_INTERVAL;
        self.stream.word_progress = 1.0;
        self.stream.cursor_visible = true;
        self.stream.cursor_phase = 0.0;
        self.stream.activity_ticks = 0;
    }

    pub(crate) fn finish_tool(&mut self, call_id: &str, is_error: bool) {
        if let Some(AgentChatEntry::Tool {
            finished,
            is_error: entry_is_error,
            ..
        }) = self.entries.iter_mut().rev().find(
            |entry| matches!(entry, AgentChatEntry::Tool { call_id: id, .. } if id == call_id),
        ) {
            *finished = true;
            *entry_is_error = is_error;
        }
    }

    pub(crate) fn finish_running_tools(&mut self, is_error: bool) {
        for entry in &mut self.entries {
            if let AgentChatEntry::Tool {
                finished,
                is_error: entry_is_error,
                ..
            } = entry
                && !*finished
            {
                *finished = true;
                *entry_is_error = is_error;
            }
        }
    }

    pub(crate) fn has_running_tool_call(&self, call_id: &str, name: &str) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                entry,
                AgentChatEntry::Tool {
                    call_id: entry_call_id,
                    name: entry_name,
                    finished: false,
                    ..
                } if entry_call_id == call_id && entry_name == name
            )
        })
    }
}

fn latest_assistant_after_last_user(entries: &[AgentChatEntry]) -> Option<usize> {
    let turn_start = entries
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
        .unwrap_or_default();
    entries
        .iter()
        .enumerate()
        .skip(turn_start)
        .filter_map(|(index, entry)| {
            matches!(
                entry,
                AgentChatEntry::Message {
                    role: AgentChatRole::Assistant,
                    ..
                }
            )
            .then_some(index)
        })
        .next_back()
}

fn reveal_units_for_backlog(bytes: usize) -> usize {
    match bytes {
        0..=96 => 1,
        97..=256 => 2,
        257..=512 => 4,
        _ => 8,
    }
}

fn reveal_prefix_len(text: &str, max_units: usize, settled: bool) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut units = 0;
    let mut inside_unit = false;
    for (index, character) in text.char_indices() {
        if character.is_whitespace() {
            if inside_unit {
                units += 1;
                inside_unit = false;
            }
        } else {
            if units >= max_units {
                return index;
            }
            inside_unit = true;
        }
    }

    if settled || (!inside_unit && units > 0) {
        text.len()
    } else if units > 0 {
        text.char_indices()
            .rev()
            .find_map(|(index, character)| {
                character
                    .is_whitespace()
                    .then_some(index + character.len_utf8())
            })
            .unwrap_or_default()
    } else if text.chars().count() > 64 {
        text.char_indices()
            .nth(64)
            .map(|(index, _)| index)
            .unwrap_or(text.len())
    } else {
        0
    }
}

fn split_assistant_follow_ups(response: &str) -> (String, Option<Vec<String>>) {
    let Some(section_start) = response.rfind(FOLLOW_UP_SECTION_START) else {
        return (response.to_string(), None);
    };
    if section_start > 0
        && !response[..section_start]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        return (response.to_string(), None);
    }

    let visible_answer = response[..section_start].trim_end().to_string();
    let metadata = &response[section_start + FOLLOW_UP_SECTION_START.len()..];
    let Some(section_end) = metadata.find(FOLLOW_UP_SECTION_END) else {
        return (visible_answer, Some(Vec::new()));
    };
    if !metadata[section_end + FOLLOW_UP_SECTION_END.len()..]
        .trim()
        .is_empty()
    {
        return (response.to_string(), None);
    }

    let payload = metadata[..section_end].trim();
    let parsed = serde_json::from_str::<Vec<String>>(payload).unwrap_or_default();
    let mut follow_ups = Vec::with_capacity(MAX_FOLLOW_UPS);
    for candidate in parsed {
        let normalized = candidate.split_whitespace().collect::<Vec<_>>().join(" ");
        let bounded = bounded_text(&normalized, MAX_FOLLOW_UP_CHARS);
        if bounded.is_empty()
            || follow_ups
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&bounded))
        {
            continue;
        }
        follow_ups.push(bounded);
        if follow_ups.len() == MAX_FOLLOW_UPS {
            break;
        }
    }
    (visible_answer, Some(follow_ups))
}

#[cfg(test)]
mod tests;
