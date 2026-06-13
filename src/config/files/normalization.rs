use crate::config::themes::{default_custom_themes, is_known_default_hyperliquid_theme};
use crate::config::{
    AccountProfile, KeroseneConfig, default_layout_ratios, default_market_slippage_pct,
    new_secret_id, normalize_alfred_popup_scale, normalize_chart_chromatic_aberration_strength,
    normalize_chart_dotted_background_opacity, normalize_chart_edge_blur_strength,
    normalize_chart_fisheye_strength, normalize_chart_hud_order_sound_volume,
    normalize_market_slippage_pct, normalize_pane_border_thickness, normalize_pane_corner_radius,
    normalize_ui_scale, prune_legacy_unsupported_pane_layout, push_config_warning,
};
use std::collections::HashSet;
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Loaded Config Normalization
// ---------------------------------------------------------------------------

pub(super) fn normalize_loaded_config(config: &mut KeroseneConfig) {
    migrate_read_data_provider(config);
    merge_default_themes(config);
    ensure_layout_ratios(config);
    prune_unsupported_pane_layouts(config);
    normalize_market_slippage(config);
    normalize_pane_chrome(config);
    normalize_fonts(config);
    migrate_legacy_single_account(config);
    ensure_unique_account_secret_ids(config);
    apply_pending_keychain_profile_deletions(config);
    ensure_account_profile(config);
    clamp_active_account(config);
}

fn migrate_read_data_provider(config: &mut KeroseneConfig) {
    if config.read_data_provider == crate::config::ReadDataProvider::Hyperliquid
        && config.chart_backfill_source == crate::config::ChartBackfillSource::Hydromancer
    {
        config.read_data_provider = crate::config::ReadDataProvider::Hydromancer;
    }
    config.chart_backfill_source = config.read_data_provider.chart_backfill_source();
}

fn prune_unsupported_pane_layouts(config: &mut KeroseneConfig) {
    config.pane_layout = config
        .pane_layout
        .take()
        .and_then(prune_legacy_unsupported_pane_layout);

    for layout in &mut config.saved_layouts {
        layout.pane_layout = layout
            .pane_layout
            .take()
            .and_then(prune_legacy_unsupported_pane_layout);
    }
}

fn normalize_market_slippage(config: &mut KeroseneConfig) {
    config.market_slippage_pct = normalized_market_slippage_pct(config.market_slippage_pct);

    for layout in &mut config.saved_layouts {
        layout.market_slippage_pct = normalized_market_slippage_pct(layout.market_slippage_pct);
    }
}

fn normalized_market_slippage_pct(value: f64) -> f64 {
    normalize_market_slippage_pct(value).unwrap_or_else(default_market_slippage_pct)
}

fn normalize_pane_chrome(config: &mut KeroseneConfig) {
    config.ui_scale = normalize_ui_scale(config.ui_scale);
    config.alfred_popup_scale = normalize_alfred_popup_scale(config.alfred_popup_scale);
    if config.chart_hollow_candles
        && config.chart_hollow_candle_mode == crate::config::ChartHollowCandleMode::Off
    {
        config.chart_hollow_candle_mode = crate::config::ChartHollowCandleMode::Up;
    }
    config.chart_hollow_candles = false;
    config.chart_dotted_background_opacity =
        normalize_chart_dotted_background_opacity(config.chart_dotted_background_opacity);
    config.chart_fisheye_strength = normalize_chart_fisheye_strength(config.chart_fisheye_strength);
    config.chart_chromatic_aberration_strength =
        normalize_chart_chromatic_aberration_strength(config.chart_chromatic_aberration_strength);
    config.chart_edge_blur_strength =
        normalize_chart_edge_blur_strength(config.chart_edge_blur_strength);
    config.chart_hud_order_sound_volume =
        normalize_chart_hud_order_sound_volume(config.chart_hud_order_sound_volume);
    config.pane_border_thickness = normalize_pane_border_thickness(config.pane_border_thickness);
    config.pane_corner_radius = normalize_pane_corner_radius(config.pane_corner_radius);
    config.widget_padding = config.widget_padding.clone().normalized();
    for layout in &mut config.saved_layouts {
        layout.widget_padding = layout.widget_padding.clone().normalized();
    }
}

fn normalize_fonts(config: &mut KeroseneConfig) {
    config.custom_fonts = crate::config::normalize_custom_fonts(config.custom_fonts.clone());
    config.display_font =
        crate::config::normalize_display_font(config.display_font.clone(), &config.custom_fonts);
    config.monospace_font =
        crate::config::normalize_display_font(config.monospace_font.clone(), &config.custom_fonts);
}

fn merge_default_themes(config: &mut KeroseneConfig) {
    for default_theme in default_custom_themes() {
        if let Some(existing) = config
            .custom_themes
            .iter_mut()
            .find(|theme| theme.name == default_theme.name)
        {
            if existing.name == "Hyperliquid" && is_known_default_hyperliquid_theme(existing) {
                *existing = default_theme.clone();
                continue;
            }
            if existing.chart_bull.is_none() {
                existing.chart_bull = default_theme.chart_bull.clone();
            }
            if existing.chart_bear.is_none() {
                existing.chart_bear = default_theme.chart_bear.clone();
            }
            if existing.name == "Kerosene"
                && existing.success.eq_ignore_ascii_case("#35D07F")
                && existing.danger.eq_ignore_ascii_case("#FF4D4D")
            {
                existing.success = default_theme.success.clone();
                existing.danger = default_theme.danger.clone();
            }
            if existing.name == "Hyperliquid" && existing.background.eq_ignore_ascii_case("#072723")
            {
                existing.background = default_theme.background.clone();
            }
        } else {
            config.custom_themes.push(default_theme);
        }
    }
}

fn ensure_layout_ratios(config: &mut KeroseneConfig) {
    if config.layout_ratios.is_empty() {
        config.layout_ratios = default_layout_ratios();
    }
}

fn migrate_legacy_single_account(config: &mut KeroseneConfig) {
    if !config.accounts.is_empty() {
        migrate_legacy_agent_key_into_active_account(config);
        return;
    }

    if config.wallet_address.is_empty() && config.agent_key.is_empty() {
        return;
    }

    config.accounts.push(AccountProfile {
        secret_id: new_secret_id(),
        name: "Main Trading".to_string(),
        wallet_address: config.wallet_address.clone(),
        agent_key: config.agent_key.clone(),
        hydromancer_api_key: String::new().into(),
    });
    config.wallet_address.clear();
    config.agent_key.zeroize();
}

fn migrate_legacy_agent_key_into_active_account(config: &mut KeroseneConfig) {
    if config.agent_key.trim().is_empty() || config.accounts.is_empty() {
        return;
    }

    let active_index = config.active_account_index.min(config.accounts.len() - 1);
    if config.accounts[active_index].agent_key.trim().is_empty() {
        config.accounts[active_index].agent_key = config.agent_key.clone();
    }
    config.agent_key.zeroize();
}

fn ensure_account_profile(config: &mut KeroseneConfig) {
    if config.accounts.is_empty() {
        config.accounts.push(AccountProfile {
            secret_id: new_secret_id(),
            name: "Main Trading".to_string(),
            wallet_address: String::new(),
            agent_key: String::new().into(),
            hydromancer_api_key: String::new().into(),
        });
    }
}

fn ensure_unique_account_secret_ids(config: &mut KeroseneConfig) {
    let mut seen = HashSet::new();
    let mut repaired_any = false;
    let mut ambiguous_cleanup_ids = HashSet::new();
    let mut account_scoped_renames = Vec::new();

    for profile in &mut config.accounts {
        let original = profile.secret_id.clone();
        let trimmed = original.trim().to_string();
        if trimmed.is_empty() || seen.contains(&trimmed) {
            if !trimmed.is_empty() {
                ambiguous_cleanup_ids.insert(trimmed.clone());
            }
            let new_id = unique_new_secret_id(&seen);
            profile.secret_id = new_id.clone();
            seen.insert(new_id);
            repaired_any = true;
            if original.trim().is_empty() || original != trimmed {
                account_scoped_renames.push((original, profile.secret_id.clone()));
            }
            continue;
        }

        if original != trimmed {
            profile.secret_id = trimmed.clone();
            account_scoped_renames.push((original, trimmed.clone()));
            repaired_any = true;
        }
        seen.insert(trimmed);
    }

    remap_account_scoped_config_state(config, &account_scoped_renames);
    if remove_ambiguous_pending_profile_deletions(config, &ambiguous_cleanup_ids) {
        config.secret_cleanup_state_dirty = true;
        push_config_warning(
            "Skipped pending credential cleanup for repaired duplicate account identifiers; remove stale credentials from storage settings if prompted."
                .to_string(),
        );
    }
    if repaired_any {
        push_config_warning(
            "Blank or duplicate account profile identifiers were repaired. If credentials for a repaired account do not unlock, re-enter and save that account's credentials."
                .to_string(),
        );
    }
}

fn unique_new_secret_id(seen: &HashSet<String>) -> String {
    loop {
        let secret_id = new_secret_id();
        if !seen.contains(&secret_id) {
            return secret_id;
        }
    }
}

fn remap_account_scoped_config_state(config: &mut KeroseneConfig, renames: &[(String, String)]) {
    for (old, new) in renames {
        if old == new {
            continue;
        }
        if let Some(hidden) = config.hidden_positions_by_account.remove(old) {
            config
                .hidden_positions_by_account
                .entry(new.clone())
                .or_insert(hidden);
        }
        if let Some(entries) = config.journal_entries_by_account.remove(old) {
            config
                .journal_entries_by_account
                .entry(new.clone())
                .or_insert(entries);
        }
    }
}

fn remove_ambiguous_pending_profile_deletions(
    config: &mut KeroseneConfig,
    ambiguous_cleanup_ids: &HashSet<String>,
) -> bool {
    if ambiguous_cleanup_ids.is_empty() || config.pending_keychain_profile_deletions.is_empty() {
        return false;
    }

    let original_len = config.pending_keychain_profile_deletions.len();
    config
        .pending_keychain_profile_deletions
        .retain(|secret_id| !ambiguous_cleanup_ids.contains(secret_id.trim()));
    config.pending_keychain_profile_deletions.len() != original_len
}

pub(super) fn apply_pending_keychain_profile_deletions(config: &mut KeroseneConfig) {
    let mut changed = normalize_pending_keychain_profile_deletions(config);
    if config.pending_keychain_profile_deletions.is_empty() || config.accounts.is_empty() {
        if changed {
            config.secret_cleanup_state_dirty = true;
        }
        return;
    }

    let pending: HashSet<&str> = config
        .pending_keychain_profile_deletions
        .iter()
        .map(String::as_str)
        .collect();
    let active_index = config.active_account_index;
    let active_removed = config
        .accounts
        .get(active_index)
        .is_some_and(|profile| pending.contains(profile.secret_id.as_str()));
    let removed_before_active = config
        .accounts
        .iter()
        .enumerate()
        .filter(|(index, profile)| {
            *index < active_index && pending.contains(profile.secret_id.as_str())
        })
        .count();
    let accounts_len = config.accounts.len();
    let hidden_positions_len = config.hidden_positions_by_account.len();
    let journal_entries_len = config.journal_entries_by_account.len();

    config
        .accounts
        .retain(|profile| !pending.contains(profile.secret_id.as_str()));
    config
        .hidden_positions_by_account
        .retain(|account_key, _| !pending.contains(account_key.as_str()));
    config
        .journal_entries_by_account
        .retain(|account_key, _| !pending.contains(account_key.as_str()));

    changed |= config.accounts.len() != accounts_len
        || config.hidden_positions_by_account.len() != hidden_positions_len
        || config.journal_entries_by_account.len() != journal_entries_len;

    let previous_active_index = config.active_account_index;
    if active_removed {
        config.active_account_index = 0;
    } else {
        config.active_account_index = config
            .active_account_index
            .saturating_sub(removed_before_active);
    }
    changed |= config.active_account_index != previous_active_index;

    if changed {
        config.secret_cleanup_state_dirty = true;
    }
}

fn normalize_pending_keychain_profile_deletions(config: &mut KeroseneConfig) -> bool {
    let mut seen = HashSet::new();
    let mut changed = false;
    config
        .pending_keychain_profile_deletions
        .retain_mut(|secret_id| {
            let trimmed = secret_id.trim().to_string();
            if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
                changed = true;
                return false;
            }
            if *secret_id != trimmed {
                *secret_id = trimmed;
                changed = true;
            }
            true
        });
    changed
}

fn clamp_active_account(config: &mut KeroseneConfig) {
    if config.active_account_index >= config.accounts.len() {
        config.active_account_index = 0;
    }
}

#[cfg(test)]
mod tests;
