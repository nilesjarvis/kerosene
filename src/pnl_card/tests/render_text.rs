use super::*;

#[test]
fn render_text_default_card_uses_leveraged_percent_usd_and_private_context() {
    let state = position_state("BTC");
    let denomination = DisplayDenominationContext::default();
    let render_text = pnl_card_render_text(&state, &sample_metrics(), &denomination);

    assert_eq!(render_text.ticker, "BTC");
    assert_eq!(render_text.leverage_display, "20x");
    assert_eq!(render_text.primary_value, "+50.14%");
    assert_eq!(render_text.percent_mode_label, "By leverage");
    assert_eq!(render_text.secondary_value, Some("+$1,076.19".to_string()));
    assert_eq!(render_text.entry_display, "82,5xx");
    assert_eq!(render_text.exit_display, "84,6xx");
    assert_eq!(render_text.context, "Short position");
}

#[test]
fn render_text_can_show_asset_move_only_with_exact_prices_and_position_size() {
    let mut state = position_state("BTC");
    state.display_mode = PnlCardDisplayMode::PercentOnly;
    state.percent_mode = PnlCardPercentMode::AssetMove;
    state.obscure_prices = false;
    state.show_position_size = true;

    let denomination = DisplayDenominationContext::default();
    let render_text = pnl_card_render_text(&state, &sample_metrics(), &denomination);

    assert_eq!(render_text.primary_value, "+2.51%");
    assert_eq!(render_text.percent_mode_label, "Asset move");
    assert_eq!(render_text.secondary_value, None);
    assert_eq!(render_text.entry_display, "82,543.2");
    assert_eq!(render_text.exit_display, "84,612.8");
    assert_eq!(render_text.context, "Short 0.52 BTC");
}

#[test]
fn render_text_can_show_usd_only_without_secondary_value() {
    let mut state = position_state("ETH");
    let mut metrics = sample_metrics();
    state.display_mode = PnlCardDisplayMode::UsdOnly;
    metrics.upnl = -42.5;

    let denomination = DisplayDenominationContext::default();
    let render_text = pnl_card_render_text(&state, &metrics, &denomination);

    assert_eq!(render_text.primary_value, "-$42.50");
    assert_eq!(render_text.secondary_value, None);
}

#[test]
fn render_text_preserves_usd_when_percent_basis_is_missing() {
    let state = summary_state();
    let mut metrics = sample_metrics();
    metrics.asset_move_pct = None;
    metrics.leveraged_pct = None;

    let denomination = DisplayDenominationContext::default();
    let render_text = pnl_card_render_text(&state, &metrics, &denomination);

    assert_eq!(render_text.primary_value, "--%");
    assert_eq!(render_text.secondary_value, Some("+$1,076.19".to_string()));
}

#[test]
fn pnl_card_context_hides_position_size_by_default() {
    let mut state = position_state("BTC");
    let metrics = sample_metrics();

    assert_eq!(pnl_card_context_display(&state, &metrics), "Short position");

    state.show_position_size = true;

    assert_eq!(pnl_card_context_display(&state, &metrics), "Short 0.52 BTC");
}

#[test]
fn summary_context_is_not_replaced_by_position_privacy_text() {
    let state = summary_state();
    let mut metrics = sample_metrics();
    metrics.context = "3 open positions".to_string();
    metrics.private_context = None;

    assert_eq!(
        pnl_card_context_display(&state, &metrics),
        "3 open positions"
    );
}
