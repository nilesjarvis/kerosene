use super::SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT;

pub(super) fn html_to_plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len().min(SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT));
    let mut in_tag = false;
    let mut tag_buf = String::new();
    let mut skip_until: Option<&'static str> = None;
    let mut entity = String::new();
    let mut in_entity = false;

    for ch in html.chars() {
        if let Some(end_tag) = skip_until {
            tag_buf.push(ch.to_ascii_lowercase());
            if tag_buf.ends_with(end_tag) {
                skip_until = None;
                tag_buf.clear();
                text.push(' ');
            } else if tag_buf.len() > end_tag.len() + 32 {
                tag_buf = tag_buf
                    .chars()
                    .rev()
                    .take(end_tag.len())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
            }
            continue;
        }

        if in_tag {
            if ch == '>' {
                let tag = tag_buf.trim().to_ascii_lowercase();
                if tag.starts_with("script") {
                    skip_until = Some("</script>");
                    tag_buf.clear();
                } else if tag.starts_with("style") {
                    skip_until = Some("</style>");
                    tag_buf.clear();
                } else {
                    if tag.starts_with("br")
                        || tag.starts_with("/p")
                        || tag.starts_with("/div")
                        || tag.starts_with("/tr")
                        || tag.starts_with("/table")
                    {
                        text.push(' ');
                    }
                    tag_buf.clear();
                }
                in_tag = false;
            } else {
                tag_buf.push(ch);
            }
            continue;
        }

        if in_entity {
            if ch == ';' {
                text.push_str(&decode_html_entity(&entity));
                entity.clear();
                in_entity = false;
            } else if entity.len() < 16 {
                entity.push(ch);
            } else {
                text.push('&');
                text.push_str(&entity);
                entity.clear();
                in_entity = false;
                text.push(ch);
            }
            continue;
        }

        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
                text.push(' ');
            }
            '&' => {
                in_entity = true;
                entity.clear();
            }
            _ => text.push(ch),
        }
    }

    normalize_filing_text(&text)
}

fn decode_html_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" | "160" => " ".to_string(),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| format!("&{entity};"))
        }
        _ if entity.starts_with('#') => entity[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| format!("&{entity};")),
        _ => format!("&{entity};"),
    }
}

fn normalize_filing_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(SEC_FILING_SUMMARY_CACHE_TEXT_LIMIT));
    let mut last_was_space = true;
    for ch in text.chars() {
        let normalized = match ch {
            '\u{2013}' | '\u{2014}' => '-',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            _ => ch,
        };
        if normalized.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(normalized);
            last_was_space = false;
        }
    }
    out.trim().to_string()
}

pub(super) fn summarize_filing_text(text: &str) -> (Option<String>, Vec<String>) {
    let headline = filing_headline(text);
    let mut highlights = Vec::new();
    for keywords in [
        &["revenue", "net sales", "sales"][..],
        &["net income", "net loss"][..],
        &["diluted", "eps", "earnings per share"][..],
        &["gross margin", "operating margin"][..],
        &["operating income", "operating loss"][..],
        &["cash flow", "free cash flow", "cash and equivalents"][..],
        &["guidance", "outlook", "expects", "forecast"][..],
        &["dividend", "repurchase", "buyback"][..],
    ] {
        if let Some(snippet) = first_relevant_snippet(text, keywords, &highlights) {
            highlights.push(snippet);
        }
        if highlights.len() >= 5 {
            break;
        }
    }

    if highlights.is_empty()
        && let Some(fallback) = fallback_filing_snippet(text, headline.as_deref())
    {
        highlights.push(fallback);
    }

    (headline, highlights)
}

fn filing_headline(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    for needle in ["reports", "announces", "financial results", "quarter"] {
        if let Some(index) = lower.find(needle) {
            let snippet = snippet_around(text, index, 90, 130);
            if snippet_has_numbers_or_reporting_words(&snippet) {
                return Some(trim_summary_snippet(&snippet, 132));
            }
        }
    }
    fallback_filing_snippet(text, None)
}

fn first_relevant_snippet(text: &str, keywords: &[&str], existing: &[String]) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let mut best: Option<(i32, String)> = None;
    for keyword in keywords {
        let mut cursor = 0;
        while let Some(relative_index) = lower[cursor..].find(keyword) {
            let index = cursor + relative_index;
            let snippet = trim_summary_snippet(&snippet_around(text, index, 70, 170), 150);
            cursor = index + keyword.len();
            if snippet_has_numbers_or_reporting_words(&snippet)
                && !snippet_is_boilerplate(&snippet)
                && !summary_snippet_seen(&snippet, existing)
            {
                let score = summary_snippet_score(&snippet, keywords);
                if best
                    .as_ref()
                    .is_none_or(|(best_score, _)| score > *best_score)
                {
                    best = Some((score, snippet));
                }
            }
        }
    }
    best.map(|(_, snippet)| snippet)
}

fn fallback_filing_snippet(text: &str, exclude: Option<&str>) -> Option<String> {
    for chunk in text.split(['.', ';']) {
        let snippet = trim_summary_snippet(chunk, 140);
        if snippet.len() > 35
            && !snippet_is_boilerplate(&snippet)
            && exclude
                .is_none_or(|excluded| !summary_snippet_seen(&snippet, &[excluded.to_string()]))
        {
            return Some(snippet);
        }
    }
    None
}

fn snippet_around(text: &str, index: usize, before: usize, after: usize) -> String {
    let start = previous_summary_delimiter(text, index)
        .map(|pos| pos + 1)
        .unwrap_or_else(|| previous_char_boundary(text, index.saturating_sub(before)));
    let end = next_summary_delimiter(text, index)
        .map(|pos| pos + 1)
        .unwrap_or_else(|| next_char_boundary(text, (index + after).min(text.len())));
    text[start..end].to_string()
}

fn previous_summary_delimiter(text: &str, before: usize) -> Option<usize> {
    let mut found = None;
    for (index, character) in text.char_indices() {
        if index >= before {
            break;
        }
        if summary_delimiter_at(text, index, character, true) {
            found = Some(index);
        }
    }
    found
}

fn next_summary_delimiter(text: &str, from: usize) -> Option<usize> {
    text.char_indices()
        .skip_while(|(index, _)| *index < from)
        .find_map(|(index, character)| {
            summary_delimiter_at(text, index, character, false).then_some(index)
        })
}

fn summary_delimiter_at(text: &str, index: usize, character: char, include_colon: bool) -> bool {
    match character {
        ';' => true,
        ':' => include_colon,
        '.' => {
            let previous_is_digit = text[..index]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit());
            let next_index = index + character.len_utf8();
            let next_is_digit = text[next_index..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit());
            !(previous_is_digit && next_is_digit)
        }
        _ => false,
    }
}

fn previous_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn trim_summary_snippet(snippet: &str, max_chars: usize) -> String {
    let snippet = normalize_filing_text(snippet)
        .trim_matches(['-', ':', ';', '.'])
        .trim()
        .to_string();
    if snippet.chars().count() <= max_chars {
        return snippet;
    }

    let mut out = snippet
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    while out.chars().last().is_some_and(char::is_whitespace) {
        out.pop();
    }
    out.push_str("...");
    out
}

fn snippet_has_numbers_or_reporting_words(snippet: &str) -> bool {
    snippet.chars().any(|ch| ch.is_ascii_digit())
        || snippet.to_ascii_lowercase().contains("quarter")
        || snippet.to_ascii_lowercase().contains("year")
}

fn summary_snippet_score(snippet: &str, keywords: &[&str]) -> i32 {
    let lower = snippet.to_ascii_lowercase();
    let mut score = 0;
    if snippet.contains('$') || lower.contains("billion") || lower.contains("million") {
        score += 30;
    }
    if snippet.contains('%') {
        score += 12;
    }
    if lower.contains("total revenue")
        || lower.contains("revenues increased")
        || lower.contains("revenue was")
        || lower.contains("net sales")
        || lower.contains("diluted eps")
        || lower.contains("diluted earnings per share")
        || lower.contains("earnings per share")
        || lower.contains("cash flow")
        || lower.contains("outlook")
        || lower.contains("expects")
        || lower.contains("guidance")
    {
        score += 25;
    }
    if lower.contains("gaap") || lower.contains("non-gaap") {
        score += 8;
    }
    for keyword in keywords {
        if lower.contains(keyword) {
            score += 4;
        }
    }
    if lower.contains("highlights 03") || lower.contains("photos & charts") {
        score -= 40;
    }
    if lower.contains("forward-looking") {
        score -= 50;
    }
    score
}

fn snippet_is_boilerplate(snippet: &str) -> bool {
    let lower = snippet.to_ascii_lowercase();
    lower.contains("forward-looking")
        || lower.contains("safe harbor")
        || lower.contains("non-gaap financial measures")
        || lower.contains("reconciliation")
        || lower.contains("conference call")
        || lower.contains("investor relations")
        || lower.contains("table of contents")
        || lower.contains("photos & charts")
        || lower.contains("additional information")
}

fn summary_snippet_seen(snippet: &str, existing: &[String]) -> bool {
    let normalized = snippet
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .take(80)
        .collect::<String>();
    existing.iter().any(|item| {
        item.chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .map(|ch| ch.to_ascii_lowercase())
            .take(80)
            .collect::<String>()
            == normalized
    })
}

#[cfg(test)]
mod tests;
