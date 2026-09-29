use super::styles::with_alpha;
use crate::app_fonts;
use crate::message::Message;
use iced::widget::{Column, markdown, rich_text, text};
use iced::{Border, Color, Element, Padding, Theme};

#[derive(Debug, Clone, Copy)]
struct AgentMarkdownViewer;

impl<'a> markdown::Viewer<'a, Message> for AgentMarkdownViewer {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::AgentOpenLink(url.into())
    }
}

pub(super) fn agent_streaming_markdown<'a>(
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

pub(super) fn agent_markdown_settings(theme: &Theme) -> markdown::Settings {
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

#[cfg(test)]
mod tests;
