//! Cross-registry consistency for user-addable widgets and windows.
//!
//! Every widget/window offered by the add-widget menu must also be reachable
//! from the Alfred command catalog, and vice versa. These two registries are
//! maintained by hand in separate modules (`add_widget_menu::body::sections`
//! and `alfred_state::catalog`), so they can silently drift: commit 749dd01
//! fixed a Session Data widget that had a `PaneKind`, an `Add…` message, and a
//! menu entry but was never added to the Alfred catalog. This test locks the
//! two registries to the same set of command messages.
//!
//! Mechanism: both registries reference each command as a `Message::<Variant>`
//! literal (and, today, reference no other messages), so we compare the set of
//! `Message::` identifiers that appear in each registry's source text. The
//! sources are embedded at compile time with `include_str!`, so the test is
//! hermetic and needs no `TradingTerminal` instance.

use std::collections::BTreeSet;

// Add-widget menu section sources. MUST stay in sync with the `mod` list in
// `add_widget_menu/body/sections.rs`; `menu_sections_cover_all_modules` fails
// if a new section module is added without updating this list, so a new
// section cannot silently escape the consistency check below.
const MENU_SECTIONS: &[(&str, &str)] = &[
    (
        "account",
        include_str!("../../add_widget_menu/body/sections/account.rs"),
    ),
    (
        "charts",
        include_str!("../../add_widget_menu/body/sections/charts.rs"),
    ),
    (
        "feeds",
        include_str!("../../add_widget_menu/body/sections/feeds.rs"),
    ),
    (
        "tools",
        include_str!("../../add_widget_menu/body/sections/tools.rs"),
    ),
    (
        "windows",
        include_str!("../../add_widget_menu/body/sections/windows.rs"),
    ),
];

const SECTIONS_MODULE: &str = include_str!("../../add_widget_menu/body/sections.rs");

// Alfred command catalog sources (widgets + windows).
const CATALOG_SOURCES: &[&str] = &[include_str!("widgets.rs"), include_str!("windows.rs")];

/// Collect every `Message::<Variant>` identifier that appears in `src`.
fn message_variants(src: &str) -> BTreeSet<String> {
    const MARKER: &str = "Message::";
    let mut out = BTreeSet::new();
    let mut rest = src;
    while let Some(pos) = rest.find(MARKER) {
        let after = &rest[pos + MARKER.len()..];
        let ident: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        rest = &after[ident.len()..];
        if !ident.is_empty() {
            out.insert(ident);
        }
    }
    out
}

#[test]
fn menu_sections_cover_all_section_modules() {
    let declared: BTreeSet<&str> = SECTIONS_MODULE
        .lines()
        .filter_map(|line| line.trim().strip_prefix("mod ")?.strip_suffix(';'))
        .collect();
    let embedded: BTreeSet<&str> = MENU_SECTIONS.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        declared, embedded,
        "add-widget menu section modules changed; update MENU_SECTIONS in this test \
         to match `add_widget_menu/body/sections.rs`"
    );
}

#[test]
fn add_widget_menu_and_alfred_catalog_expose_the_same_commands() {
    let menu: BTreeSet<String> = MENU_SECTIONS
        .iter()
        .flat_map(|(_, src)| message_variants(src))
        .collect();
    let catalog: BTreeSet<String> = CATALOG_SOURCES
        .iter()
        .flat_map(|src| message_variants(src))
        .collect();

    let menu_only: Vec<&String> = menu.difference(&catalog).collect();
    let catalog_only: Vec<&String> = catalog.difference(&menu).collect();

    assert!(
        menu_only.is_empty() && catalog_only.is_empty(),
        "add-widget menu and Alfred command catalog are out of sync.\n  \
         In menu but missing from Alfred catalog: {menu_only:?}\n  \
         In Alfred catalog but missing from menu: {catalog_only:?}\n  \
         Add the missing command to the other registry (see commit 749dd01)."
    );
}
