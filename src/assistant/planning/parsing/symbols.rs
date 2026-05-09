use crate::assistant::AssistantRuntimeContext;

pub(in crate::assistant::planning) fn resolve_symbol(
    mentions: &[String],
    ctx: &AssistantRuntimeContext,
) -> String {
    mentions
        .first()
        .cloned()
        .unwrap_or_else(|| ctx.active_symbol.clone())
}

pub(in crate::assistant::planning) fn pick_symbol_candidate(
    planned_symbols: &[String],
    mentions: &[String],
    ctx: &AssistantRuntimeContext,
) -> String {
    for s in planned_symbols {
        if !is_probable_usd_amount(s) {
            return s.clone();
        }
    }
    for s in mentions {
        if !is_probable_usd_amount(s) {
            return s.clone();
        }
    }
    ctx.active_symbol.clone()
}

pub(in crate::assistant::planning) fn extract_ticker_mentions(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0_usize;
    let bytes = input.as_bytes();
    while i + 2 <= bytes.len() {
        if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'{') {
            let start = i + 2;
            if let Some(rel_end) = input[start..].find('}') {
                let end = start + rel_end;
                let token = input[start..end].trim();
                if !token.is_empty()
                    && !is_probable_usd_amount(token)
                    && !out.iter().any(|t| t == token)
                {
                    out.push(token.to_string());
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }

    for token in input.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| {
            c == ',' || c == '.' || c == ';' || c == '!' || c == '?' || c == '(' || c == ')'
        });
        if !trimmed.starts_with('$') || trimmed.starts_with("${") {
            continue;
        }
        let sym = trimmed
            .trim_start_matches('$')
            .trim_matches(|c: char| c == '{' || c == '}')
            .trim();
        if sym.is_empty() {
            continue;
        }
        let valid = sym
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ':' || c == '_' || c == '-');
        if valid && !is_probable_usd_amount(sym) && !out.iter().any(|t| t.eq_ignore_ascii_case(sym))
        {
            out.push(sym.to_string());
        }
    }
    out
}

fn is_probable_usd_amount(token: &str) -> bool {
    let t = token.trim();
    if t.is_empty() {
        return false;
    }
    t.chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
}
