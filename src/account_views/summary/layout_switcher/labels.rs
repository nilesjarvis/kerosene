// ---------------------------------------------------------------------------
// Layout Switcher Labels
// ---------------------------------------------------------------------------

pub(super) const BUTTON_LABEL_CHARS: usize = 14;
pub(super) const ROW_LABEL_CHARS: usize = 24;

pub(super) fn layout_switcher_label(name: Option<&str>, max_chars: usize) -> String {
    let label = name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Layouts");
    truncate_label(label, max_chars)
}

pub(super) fn layout_switcher_button_label(name: Option<&str>, max_chars: usize) -> String {
    let Some(label) = name.map(str::trim).filter(|name| !name.is_empty()) else {
        return "Layout".to_string();
    };
    format!("Layout: {}", truncate_label(label, max_chars))
}

fn truncate_label(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }
    let prefix: String = value.chars().take(max_chars - 3).collect();
    format!("{prefix}...")
}
