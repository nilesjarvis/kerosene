use crate::config::themes::{default_custom_themes, is_known_default_hyperliquid_theme};
use crate::config::{
    AccountProfile, KeroseneConfig, default_layout_ratios, default_market_slippage_pct,
    new_secret_id, normalize_alfred_popup_scale, normalize_market_slippage_pct,
    normalize_pane_border_thickness, normalize_pane_corner_radius, normalize_ui_scale,
    prune_unsupported_pane_layout,
};
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Loaded Config Normalization
// ---------------------------------------------------------------------------

pub(super) fn normalize_loaded_config(config: &mut KeroseneConfig) {
    merge_default_themes(config);
    ensure_layout_ratios(config);
    prune_unsupported_pane_layouts(config);
    normalize_market_slippage(config);
    normalize_pane_chrome(config);
    normalize_fonts(config);
    migrate_legacy_single_account(config);
    ensure_account_profile(config);
    clamp_active_account(config);
}

fn prune_unsupported_pane_layouts(config: &mut KeroseneConfig) {
    config.pane_layout = config
        .pane_layout
        .take()
        .and_then(prune_unsupported_pane_layout);

    for layout in &mut config.saved_layouts {
        layout.pane_layout = layout
            .pane_layout
            .take()
            .and_then(prune_unsupported_pane_layout);
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
    config.pane_border_thickness = normalize_pane_border_thickness(config.pane_border_thickness);
    config.pane_corner_radius = normalize_pane_corner_radius(config.pane_corner_radius);
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
    if !config.accounts.is_empty()
        || (config.wallet_address.is_empty() && config.agent_key.is_empty())
    {
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

fn clamp_active_account(config: &mut KeroseneConfig) {
    if config.active_account_index >= config.accounts.len() {
        config.active_account_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_out_of_range_market_slippage() {
        let mut value =
            serde_json::to_value(KeroseneConfig::default()).expect("default config serializes");
        let object = value
            .as_object_mut()
            .expect("config should serialize to object");
        object.insert("market_slippage_pct".to_string(), serde_json::json!(99.0));
        object.insert(
            "saved_layouts".to_string(),
            serde_json::json!([
                {
                    "name": "bad-slippage",
                    "market_slippage_pct": 99.0,
                }
            ]),
        );
        let mut config: KeroseneConfig =
            serde_json::from_value(value).expect("test config deserializes");

        normalize_loaded_config(&mut config);

        assert_eq!(config.market_slippage_pct, default_market_slippage_pct());
        assert_eq!(
            config.saved_layouts[0].market_slippage_pct,
            default_market_slippage_pct()
        );
    }

    #[test]
    fn normalizes_out_of_range_pane_chrome() {
        let mut config = KeroseneConfig {
            ui_scale: 99.0,
            alfred_popup_scale: 99.0,
            pane_border_thickness: 99.0,
            pane_corner_radius: f32::NAN,
            ..KeroseneConfig::default()
        };

        normalize_loaded_config(&mut config);

        assert_eq!(config.ui_scale, normalize_ui_scale(99.0));
        assert_eq!(
            config.alfred_popup_scale,
            normalize_alfred_popup_scale(99.0)
        );
        assert_eq!(
            config.pane_border_thickness,
            normalize_pane_border_thickness(99.0)
        );
        assert_eq!(
            config.pane_corner_radius,
            crate::config::default_pane_corner_radius()
        );
    }

    #[test]
    fn prunes_unsupported_panes_from_loaded_layouts() {
        let mut config = KeroseneConfig {
            pane_layout: Some(crate::config::PaneLayoutConfig::Split {
                axis: crate::config::AxisConfig::Vertical,
                ratio: 0.5,
                a: Box::new(crate::config::PaneLayoutConfig::Leaf(
                    crate::config::PaneKindConfig::Chart { chart_id: 7 },
                )),
                b: Box::new(crate::config::PaneLayoutConfig::Leaf(
                    crate::config::PaneKindConfig::Unsupported,
                )),
            }),
            saved_layouts: vec![
                serde_json::from_value(serde_json::json!({
                    "name": "legacy-assistant-only",
                    "pane_layout": { "Leaf": "Assistant" }
                }))
                .expect("legacy saved layout should deserialize"),
            ],
            ..KeroseneConfig::default()
        };

        normalize_loaded_config(&mut config);

        assert_eq!(
            config.pane_layout,
            Some(crate::config::PaneLayoutConfig::Leaf(
                crate::config::PaneKindConfig::Chart { chart_id: 7 }
            ))
        );
        assert_eq!(config.saved_layouts[0].pane_layout, None);
    }
}
