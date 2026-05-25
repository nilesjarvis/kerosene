// ---------------------------------------------------------------------------
// History Summary Text
// ---------------------------------------------------------------------------

pub(super) fn compact_summary(summary: &str) -> String {
    const LIMIT: usize = 140;
    if summary.chars().count() <= LIMIT {
        return summary.to_string();
    }
    summary.chars().take(LIMIT).collect::<String>() + "..."
}

pub(super) fn hide_order_oid_references(summary: &str) -> String {
    strip_inline_oid_references(&strip_parenthesized_oid_references(summary))
}

fn strip_parenthesized_oid_references(summary: &str) -> String {
    let mut result = String::with_capacity(summary.len());
    let mut rest = summary;
    while let Some(start) = rest.find("(oid ") {
        result.push_str(rest[..start].trim_end());
        let after_prefix = &rest[start + "(oid ".len()..];
        let Some(end) = after_prefix.find(')') else {
            result.push_str(&rest[start..]);
            return result;
        };
        let order_id = &after_prefix[..end];
        if order_id.is_empty() || !order_id.chars().all(|ch| ch.is_ascii_digit()) {
            result.push_str(&rest[start..start + "(oid ".len()]);
            rest = after_prefix;
            continue;
        }
        rest = &after_prefix[end + 1..];
    }
    result.push_str(rest);
    result
}

fn strip_inline_oid_references(summary: &str) -> String {
    let mut words = summary.split_whitespace().peekable();
    let mut stripped = Vec::new();

    while let Some(word) = words.next() {
        if word.eq_ignore_ascii_case("oid")
            && let Some(next) = words.peek()
            && let Some(suffix) = numeric_token_suffix(next)
        {
            words.next();
            stripped.push(format!("order{suffix}"));
            continue;
        }

        if let Some(suffix) = oid_assignment_suffix(word) {
            stripped.push(format!("order{suffix}"));
            continue;
        }

        stripped.push(word.to_string());
    }

    stripped.join(" ")
}

fn oid_assignment_suffix(word: &str) -> Option<&str> {
    let (label, value) = word.split_once('=')?;
    if label.eq_ignore_ascii_case("oid") {
        numeric_token_suffix(value)
    } else {
        None
    }
}

fn numeric_token_suffix(value: &str) -> Option<&str> {
    let digit_end = value
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_ascii_digit()).then_some(index))
        .unwrap_or(value.len());
    (digit_end > 0).then_some(&value[digit_end..])
}
