use crate::config::themes::{
    default_custom_themes, is_known_default_bloomberg_theme, is_known_default_hyperliquid_theme,
};
use crate::config::{
    AccountProfile, KeroseneConfig, PaneKindConfig, PaneLayoutConfig, SavedLayout,
    default_layout_ratios, default_market_slippage_pct, new_secret_id,
    normalize_alfred_popup_scale, normalize_chart_chromatic_aberration_strength,
    normalize_chart_dotted_background_opacity, normalize_chart_edge_blur_strength,
    normalize_chart_fisheye_strength, normalize_chart_gradient_contrast,
    normalize_chart_hud_order_sound_volume, normalize_market_slippage_pct,
    normalize_pane_border_thickness, normalize_pane_corner_radius, normalize_pane_split_ratio,
    normalize_ui_scale, prune_legacy_unsupported_pane_layout, push_config_warning,
};
use std::collections::{BTreeMap, BTreeSet, HashSet, btree_map::Entry};
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Loaded Config Normalization
// ---------------------------------------------------------------------------

pub(super) fn normalize_loaded_config(config: &mut KeroseneConfig) {
    migrate_read_data_provider(config);
    merge_default_themes(config);
    ensure_layout_ratios(config);
    prune_unsupported_pane_layouts(config);
    normalize_canvases(config);
    repair_duplicate_chart_widget_ids(config);
    repair_duplicate_non_chart_widget_ids(config);
    normalize_market_slippage(config);
    normalize_pane_chrome(config);
    normalize_fonts(config);
    migrate_legacy_single_account(config);
    ensure_unique_account_secret_ids(config);
    apply_pending_keychain_profile_deletions(config);
    ensure_account_profile(config);
    clamp_active_account(config);
}

pub(crate) fn normalize_imported_saved_layout(layout: &mut SavedLayout) {
    layout.pane_layout = layout
        .pane_layout
        .take()
        .and_then(prune_legacy_unsupported_pane_layout);
    for canvas in &mut layout.canvases {
        canvas.pane_layout = canvas
            .pane_layout
            .take()
            .and_then(prune_legacy_unsupported_pane_layout);
    }

    let repaired_canvas_ids = normalize_canvas_list(&mut layout.canvases);
    normalize_layout_ratio_values(&mut layout.layout_ratios);
    if let Some(pane_layout) = &mut layout.pane_layout {
        pane_layout.normalize_split_ratios();
    }
    for canvas in &mut layout.canvases {
        if let Some(pane_layout) = &mut canvas.pane_layout {
            pane_layout.normalize_split_ratios();
        }
    }

    let repaired_chart_ids = repair_duplicate_chart_widget_ids_for_layout(
        &mut layout.charts,
        &mut layout.spaghetti_charts,
        layout.pane_layout.as_mut(),
        &mut layout.canvases,
    );
    let repaired_non_chart_ids = repair_duplicate_non_chart_widget_ids_for_layout(
        &mut layout.order_books,
        &mut layout.live_watchlists,
        &mut layout.positioning_infos,
        &mut layout.session_data,
        &mut layout.x_feeds,
        layout.pane_layout.as_mut(),
        &mut layout.canvases,
    );

    layout.market_slippage_pct = normalized_market_slippage_pct(layout.market_slippage_pct);
    layout.widget_padding = layout.widget_padding.clone().normalized();

    if repaired_canvas_ids {
        push_config_warning("Duplicate Canvas identifiers were repaired.".to_string());
    }
    if repaired_chart_ids {
        push_config_warning(
            "Duplicate chart widget identifiers were repaired in persisted layouts.".to_string(),
        );
    }
    if repaired_non_chart_ids {
        push_config_warning(
            "Duplicate non-chart widget identifiers were repaired in persisted layouts."
                .to_string(),
        );
    }
}

fn normalize_canvases(config: &mut KeroseneConfig) {
    let mut repaired_ids = normalize_canvas_list(&mut config.canvases);
    for layout in &mut config.saved_layouts {
        repaired_ids |= normalize_canvas_list(&mut layout.canvases);
    }
    if repaired_ids {
        push_config_warning("Duplicate Canvas identifiers were repaired.".to_string());
    }
}

fn normalize_canvas_list(canvases: &mut [crate::config::CanvasConfig]) -> bool {
    let mut used_ids = BTreeSet::new();
    let mut repaired_ids = false;
    for canvas in canvases {
        if !used_ids.insert(canvas.id) {
            canvas.id = next_unused_widget_id(&used_ids);
            used_ids.insert(canvas.id);
            repaired_ids = true;
        }
        canvas.label = canvas.label.trim().to_string();
        canvas.width = normalize_canvas_extent(canvas.width, crate::config::DEFAULT_CANVAS_WIDTH);
        canvas.height =
            normalize_canvas_extent(canvas.height, crate::config::DEFAULT_CANVAS_HEIGHT);
        canvas.x = canvas.x.filter(|value| value.is_finite());
        canvas.y = canvas.y.filter(|value| value.is_finite());
    }
    repaired_ids
}

fn normalize_canvas_extent(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.max(320.0)
    } else {
        fallback
    }
}

fn repair_duplicate_chart_widget_ids(config: &mut KeroseneConfig) {
    let mut repaired_any = repair_duplicate_chart_widget_ids_for_layout(
        &mut config.charts,
        &mut config.spaghetti_charts,
        config.pane_layout.as_mut(),
        &mut config.canvases,
    );

    for layout in &mut config.saved_layouts {
        repaired_any |= repair_duplicate_chart_widget_ids_for_layout(
            &mut layout.charts,
            &mut layout.spaghetti_charts,
            layout.pane_layout.as_mut(),
            &mut layout.canvases,
        );
    }

    if repaired_any {
        push_config_warning(
            "Duplicate chart widget identifiers were repaired in persisted layouts.".to_string(),
        );
    }
}

fn repair_duplicate_chart_widget_ids_for_layout(
    chart_configs: &mut Vec<crate::config::ChartConfig>,
    spaghetti_configs: &mut Vec<crate::config::SpaghettiChartConfig>,
    pane_layout: Option<&mut PaneLayoutConfig>,
    canvases: &mut [crate::config::CanvasConfig],
) -> bool {
    let (chart_templates, mut repaired_any) =
        deduplicate_widget_configs(chart_configs, |config| config.id);
    let (spaghetti_templates, spaghetti_configs_repaired) =
        deduplicate_widget_configs(spaghetti_configs, |config| config.id);
    repaired_any |= spaghetti_configs_repaired;

    let mut reserved_charts = chart_templates.keys().copied().collect::<BTreeSet<_>>();
    let mut reserved_spaghetti = spaghetti_templates.keys().copied().collect::<BTreeSet<_>>();
    if let Some(layout) = pane_layout.as_deref() {
        collect_chart_pane_ids(layout, &mut reserved_charts, &mut reserved_spaghetti);
    }
    for canvas in canvases.iter() {
        if let Some(layout) = &canvas.pane_layout {
            collect_chart_pane_ids(layout, &mut reserved_charts, &mut reserved_spaghetti);
        }
    }

    let mut seen_charts = BTreeSet::new();
    let mut seen_spaghetti = BTreeSet::new();
    if let Some(layout) = pane_layout {
        repaired_any |= repair_duplicate_chart_pane_ids(
            layout,
            &mut seen_charts,
            &mut seen_spaghetti,
            &mut reserved_charts,
            &mut reserved_spaghetti,
            &chart_templates,
            &spaghetti_templates,
            chart_configs,
            spaghetti_configs,
        );
    }
    for canvas in canvases {
        if let Some(layout) = &mut canvas.pane_layout {
            repaired_any |= repair_duplicate_chart_pane_ids(
                layout,
                &mut seen_charts,
                &mut seen_spaghetti,
                &mut reserved_charts,
                &mut reserved_spaghetti,
                &chart_templates,
                &spaghetti_templates,
                chart_configs,
                spaghetti_configs,
            );
        }
    }

    repaired_any
}

fn deduplicate_widget_configs<T: Clone>(
    configs: &mut Vec<T>,
    id_for: impl Fn(&T) -> u64,
) -> (BTreeMap<u64, T>, bool) {
    let mut templates = BTreeMap::new();
    let mut unique = Vec::with_capacity(configs.len());
    let mut repaired = false;

    for config in configs.drain(..) {
        match templates.entry(id_for(&config)) {
            Entry::Vacant(entry) => {
                entry.insert(config.clone());
                unique.push(config);
            }
            Entry::Occupied(_) => repaired = true,
        }
    }
    *configs = unique;

    (templates, repaired)
}

fn collect_chart_pane_ids(
    layout: &PaneLayoutConfig,
    chart_ids: &mut BTreeSet<u64>,
    spaghetti_ids: &mut BTreeSet<u64>,
) {
    match layout {
        PaneLayoutConfig::Leaf(PaneKindConfig::Chart { chart_id }) => {
            chart_ids.insert(*chart_id);
        }
        PaneLayoutConfig::Leaf(PaneKindConfig::SpaghettiChart { spaghetti_id }) => {
            spaghetti_ids.insert(*spaghetti_id);
        }
        PaneLayoutConfig::Leaf(_) => {}
        PaneLayoutConfig::Split { a, b, .. } => {
            collect_chart_pane_ids(a, chart_ids, spaghetti_ids);
            collect_chart_pane_ids(b, chart_ids, spaghetti_ids);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn repair_duplicate_chart_pane_ids(
    layout: &mut PaneLayoutConfig,
    seen_charts: &mut BTreeSet<u64>,
    seen_spaghetti: &mut BTreeSet<u64>,
    reserved_charts: &mut BTreeSet<u64>,
    reserved_spaghetti: &mut BTreeSet<u64>,
    chart_templates: &BTreeMap<u64, crate::config::ChartConfig>,
    spaghetti_templates: &BTreeMap<u64, crate::config::SpaghettiChartConfig>,
    chart_configs: &mut Vec<crate::config::ChartConfig>,
    spaghetti_configs: &mut Vec<crate::config::SpaghettiChartConfig>,
) -> bool {
    match layout {
        PaneLayoutConfig::Leaf(PaneKindConfig::Chart { chart_id }) => {
            if seen_charts.insert(*chart_id) {
                return false;
            }
            let original_id = *chart_id;
            let replacement = next_unused_widget_id(reserved_charts);
            *chart_id = replacement;
            seen_charts.insert(replacement);
            reserved_charts.insert(replacement);
            if let Some(template) = chart_templates.get(&original_id) {
                let mut config = template.clone();
                config.id = replacement;
                chart_configs.push(config);
            }
            true
        }
        PaneLayoutConfig::Leaf(PaneKindConfig::SpaghettiChart { spaghetti_id }) => {
            if seen_spaghetti.insert(*spaghetti_id) {
                return false;
            }
            let original_id = *spaghetti_id;
            let replacement = next_unused_widget_id(reserved_spaghetti);
            *spaghetti_id = replacement;
            seen_spaghetti.insert(replacement);
            reserved_spaghetti.insert(replacement);
            if let Some(template) = spaghetti_templates.get(&original_id) {
                let mut config = template.clone();
                config.id = replacement;
                spaghetti_configs.push(config);
            }
            true
        }
        PaneLayoutConfig::Leaf(_) => false,
        PaneLayoutConfig::Split { a, b, .. } => {
            repair_duplicate_chart_pane_ids(
                a,
                seen_charts,
                seen_spaghetti,
                reserved_charts,
                reserved_spaghetti,
                chart_templates,
                spaghetti_templates,
                chart_configs,
                spaghetti_configs,
            ) | repair_duplicate_chart_pane_ids(
                b,
                seen_charts,
                seen_spaghetti,
                reserved_charts,
                reserved_spaghetti,
                chart_templates,
                spaghetti_templates,
                chart_configs,
                spaghetti_configs,
            )
        }
    }
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
    for canvas in &mut config.canvases {
        canvas.pane_layout = canvas
            .pane_layout
            .take()
            .and_then(prune_legacy_unsupported_pane_layout);
    }

    for layout in &mut config.saved_layouts {
        layout.pane_layout = layout
            .pane_layout
            .take()
            .and_then(prune_legacy_unsupported_pane_layout);
        for canvas in &mut layout.canvases {
            canvas.pane_layout = canvas
                .pane_layout
                .take()
                .and_then(prune_legacy_unsupported_pane_layout);
        }
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
    config.chart_gradient_contrast =
        normalize_chart_gradient_contrast(config.chart_gradient_contrast);
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
            if existing.name == "Bloomberg" && is_known_default_bloomberg_theme(existing) {
                *existing = default_theme.clone();
                continue;
            }
            if existing.chart_bull.is_none() {
                existing.chart_bull = default_theme.chart_bull.clone();
            }
            if existing.chart_bear.is_none() {
                existing.chart_bear = default_theme.chart_bear.clone();
            }
            if existing.chart_line.is_none() {
                existing.chart_line = default_theme.chart_line.clone();
            }
            if existing.chart_line_gradient.is_none() {
                existing.chart_line_gradient = default_theme.chart_line_gradient.clone();
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
    normalize_layout_ratio_values(&mut config.layout_ratios);

    if let Some(layout) = &mut config.pane_layout {
        layout.normalize_split_ratios();
    }
    for canvas in &mut config.canvases {
        if let Some(layout) = &mut canvas.pane_layout {
            layout.normalize_split_ratios();
        }
    }

    for layout in &mut config.saved_layouts {
        normalize_layout_ratio_values(&mut layout.layout_ratios);
        if let Some(pane_layout) = &mut layout.pane_layout {
            pane_layout.normalize_split_ratios();
        }
        for canvas in &mut layout.canvases {
            if let Some(pane_layout) = &mut canvas.pane_layout {
                pane_layout.normalize_split_ratios();
            }
        }
    }
}

fn normalize_layout_ratio_values(ratios: &mut [f32]) {
    for ratio in ratios {
        *ratio = normalize_pane_split_ratio(*ratio);
    }
}

#[derive(Debug, Default)]
struct NonChartWidgetIds {
    order_books: BTreeSet<u64>,
    live_watchlists: BTreeSet<u64>,
    positioning_infos: BTreeSet<u64>,
    session_data: BTreeSet<u64>,
    x_feeds: BTreeSet<u64>,
}

fn repair_duplicate_non_chart_widget_ids(config: &mut KeroseneConfig) {
    let mut repaired_any = repair_duplicate_non_chart_widget_ids_for_layout(
        &mut config.order_books,
        &mut config.live_watchlists,
        &mut config.positioning_infos,
        &mut config.session_data,
        &mut config.x_feeds,
        config.pane_layout.as_mut(),
        &mut config.canvases,
    );

    for layout in &mut config.saved_layouts {
        repaired_any |= repair_duplicate_non_chart_widget_ids_for_layout(
            &mut layout.order_books,
            &mut layout.live_watchlists,
            &mut layout.positioning_infos,
            &mut layout.session_data,
            &mut layout.x_feeds,
            layout.pane_layout.as_mut(),
            &mut layout.canvases,
        );
    }

    if repaired_any {
        push_config_warning(
            "Duplicate non-chart widget identifiers were repaired in persisted layouts."
                .to_string(),
        );
    }
}

fn repair_duplicate_non_chart_widget_ids_for_layout(
    order_books: &mut [crate::config::OrderBookConfig],
    live_watchlists: &mut [crate::config::LiveWatchlistConfig],
    positioning_infos: &mut [crate::config::PositioningInfoConfig],
    session_data: &mut [crate::config::SessionDataConfig],
    x_feeds: &mut [crate::config::XFeedConfig],
    pane_layout: Option<&mut PaneLayoutConfig>,
    canvases: &mut [crate::config::CanvasConfig],
) -> bool {
    let (order_books_repaired, order_book_ids) =
        repair_duplicate_ids(order_books, |config| &mut config.id);
    let (live_watchlists_repaired, live_watchlist_ids) =
        repair_duplicate_ids(live_watchlists, |config| &mut config.id);
    let (positioning_infos_repaired, positioning_info_ids) =
        repair_duplicate_ids(positioning_infos, |config| &mut config.id);
    let (session_data_repaired, session_data_ids) =
        repair_duplicate_ids(session_data, |config| &mut config.id);
    let (x_feeds_repaired, x_feed_ids) = repair_duplicate_ids(x_feeds, |config| &mut config.id);

    let mut repaired_any = order_books_repaired
        || live_watchlists_repaired
        || positioning_infos_repaired
        || session_data_repaired
        || x_feeds_repaired;

    let mut seen = NonChartWidgetIds::default();
    let mut reserved = NonChartWidgetIds {
        order_books: order_book_ids,
        live_watchlists: live_watchlist_ids,
        positioning_infos: positioning_info_ids,
        session_data: session_data_ids,
        x_feeds: x_feed_ids,
    };
    if let Some(pane_layout) = pane_layout {
        repaired_any |= repair_duplicate_non_chart_pane_ids(pane_layout, &mut seen, &mut reserved);
    }
    for canvas in canvases {
        if let Some(pane_layout) = &mut canvas.pane_layout {
            repaired_any |=
                repair_duplicate_non_chart_pane_ids(pane_layout, &mut seen, &mut reserved);
        }
    }

    repaired_any
}

fn repair_duplicate_ids<T>(
    items: &mut [T],
    mut id_for: impl FnMut(&mut T) -> &mut u64,
) -> (bool, BTreeSet<u64>) {
    let mut repaired = false;
    let mut used = BTreeSet::new();

    for item in items {
        let id = id_for(item);
        if !used.insert(*id) {
            *id = next_unused_widget_id(&used);
            used.insert(*id);
            repaired = true;
        }
    }

    (repaired, used)
}

fn repair_duplicate_non_chart_pane_ids(
    layout: &mut PaneLayoutConfig,
    seen: &mut NonChartWidgetIds,
    reserved: &mut NonChartWidgetIds,
) -> bool {
    match layout {
        PaneLayoutConfig::Leaf(kind) => repair_duplicate_non_chart_leaf_id(kind, seen, reserved),
        PaneLayoutConfig::Split { a, b, .. } => {
            repair_duplicate_non_chart_pane_ids(a, seen, reserved)
                | repair_duplicate_non_chart_pane_ids(b, seen, reserved)
        }
    }
}

fn repair_duplicate_non_chart_leaf_id(
    kind: &mut PaneKindConfig,
    seen: &mut NonChartWidgetIds,
    reserved: &mut NonChartWidgetIds,
) -> bool {
    match kind {
        PaneKindConfig::OrderBook { id } => {
            repair_duplicate_leaf_id(id, &mut seen.order_books, &mut reserved.order_books)
        }
        PaneKindConfig::LiveWatchlist { id } => {
            repair_duplicate_leaf_id(id, &mut seen.live_watchlists, &mut reserved.live_watchlists)
        }
        PaneKindConfig::PositioningInfo { id } => repair_duplicate_leaf_id(
            id,
            &mut seen.positioning_infos,
            &mut reserved.positioning_infos,
        ),
        PaneKindConfig::SessionData { id } => {
            repair_duplicate_leaf_id(id, &mut seen.session_data, &mut reserved.session_data)
        }
        PaneKindConfig::XFeed { id } => {
            repair_duplicate_leaf_id(id, &mut seen.x_feeds, &mut reserved.x_feeds)
        }
        _ => false,
    }
}

fn repair_duplicate_leaf_id(
    id: &mut u64,
    seen: &mut BTreeSet<u64>,
    reserved: &mut BTreeSet<u64>,
) -> bool {
    if seen.insert(*id) {
        reserved.insert(*id);
        return false;
    }

    let replacement = next_unused_widget_id(reserved);
    *id = replacement;
    seen.insert(replacement);
    reserved.insert(replacement);
    true
}

fn next_unused_widget_id(used: &BTreeSet<u64>) -> u64 {
    let mut next = 0;
    while used.contains(&next) {
        next = next.saturating_add(1);
    }
    next
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
