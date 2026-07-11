use super::*;

#[test]
fn pnl_card_window_state_debug_redacts_account_and_target() {
    const SYMBOL: &str = "private-pnl-card-symbol-sentinel";
    let state =
        PnlCardWindowState::new(PnlCardTarget::Position(SYMBOL.to_string()), test_account());

    let rendered = format!("{state:?}");

    assert!(!rendered.contains(&test_account()));
    assert!(!rendered.contains(SYMBOL));
    assert!(rendered.contains("<redacted>"));
    assert!(rendered.contains("Position(<redacted>)"));
    assert!(rendered.contains("obscure_prices: true"));
    assert_eq!(
        state.target,
        PnlCardTarget::Position(SYMBOL.to_string()),
        "diagnostic redaction must not change target identity"
    );
}

#[test]
fn pnl_card_metrics_and_render_text_debug_are_value_neutral() {
    const SYMBOL: &str = "private-pnl-metrics-symbol-sentinel";
    const LEVERAGE: &str = "private-pnl-leverage-sentinel";
    const ENTRY: &str = "private-pnl-entry-sentinel";
    const EXIT: &str = "private-pnl-exit-sentinel";
    const CONTEXT: &str = "private-pnl-position-size-sentinel";
    const PRIVATE_CONTEXT: &str = "private-pnl-context-sentinel";
    const PRIMARY: &str = "private-pnl-primary-value-sentinel";
    const SECONDARY: &str = "private-pnl-secondary-value-sentinel";
    const UPNL: f64 = 9_876_543.125;
    const ASSET_MOVE: f64 = 73.125;
    const LEVERAGED_MOVE: f64 = 812.375;

    let metrics = PnlCardMetrics {
        ticker: SYMBOL.to_string(),
        leverage_display: LEVERAGE.to_string(),
        entry_display: ENTRY.to_string(),
        exit_display: EXIT.to_string(),
        context: CONTEXT.to_string(),
        private_context: Some(PRIVATE_CONTEXT.to_string()),
        upnl: UPNL,
        asset_move_pct: Some(ASSET_MOVE),
        leveraged_pct: Some(LEVERAGED_MOVE),
    };
    let render_text = PnlCardRenderText {
        ticker: SYMBOL.to_string(),
        leverage_display: LEVERAGE.to_string(),
        primary_value: PRIMARY.to_string(),
        percent_mode_label: "By leverage",
        secondary_value: Some(SECONDARY.to_string()),
        entry_display: ENTRY.to_string(),
        exit_display: EXIT.to_string(),
        context: CONTEXT.to_string(),
    };

    let metrics_debug = format!("{metrics:?}");
    let render_text_debug = format!("{render_text:?}");
    let numeric_sentinels = [
        format!("{UPNL:?}"),
        format!("{ASSET_MOVE:?}"),
        format!("{LEVERAGED_MOVE:?}"),
    ];

    for sentinel in [SYMBOL, LEVERAGE, ENTRY, EXIT, CONTEXT, PRIVATE_CONTEXT] {
        assert!(!metrics_debug.contains(sentinel), "{metrics_debug}");
    }
    for sentinel in [SYMBOL, LEVERAGE, PRIMARY, SECONDARY, ENTRY, EXIT, CONTEXT] {
        assert!(!render_text_debug.contains(sentinel), "{render_text_debug}");
    }
    for sentinel in numeric_sentinels {
        assert!(!metrics_debug.contains(&sentinel), "{metrics_debug}");
    }
    assert!(metrics_debug.contains("<redacted>"), "{metrics_debug}");
    assert!(
        metrics_debug.contains("private_context_present: true"),
        "{metrics_debug}"
    );
    assert!(
        render_text_debug.contains("secondary_value_present: true"),
        "{render_text_debug}"
    );
}

#[test]
fn pnl_card_image_debug_reports_only_safe_shape() {
    const FILENAME: &str = "private-pnl-export-filename-sentinel.png";
    let image = PnlCardImage {
        width: 2,
        height: 1,
        rgba: vec![241, 242, 243, 244],
        png: vec![231, 232, 233],
        default_filename: FILENAME.to_string(),
    };

    let rendered = format!("{image:?}");

    assert!(rendered.contains("width: 2"), "{rendered}");
    assert!(rendered.contains("height: 1"), "{rendered}");
    assert!(rendered.contains("rgba_len: 4"), "{rendered}");
    assert!(rendered.contains("png_len: 3"), "{rendered}");
    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(!rendered.contains(FILENAME), "{rendered}");
    assert!(!rendered.contains("241, 242, 243, 244"), "{rendered}");
    assert!(!rendered.contains("231, 232, 233"), "{rendered}");
}

#[test]
fn pnl_card_palette_debug_does_not_reveal_direction_colors() {
    let palette = pnl_card_palette(
        &iced::Theme::Dark,
        iced::Color::from_rgb(0.125, 0.625, 0.875),
    );

    let rendered = format!("{palette:?}");

    assert_eq!(rendered, "PnlCardPalette(<redacted>)");
}

#[test]
fn privacy_price_display_can_be_disabled() {
    assert_eq!(privacy_price_display("82,543.2", true), "82,5xx");
    assert_eq!(privacy_price_display("82,543.2", false), "82,543.2");
}

#[test]
fn price_privacy_obscures_large_prices_to_hundreds() {
    assert_eq!(obscure_price_digits("82,543.2"), "82,5xx");
    assert_eq!(obscure_price_digits("12,345.7"), "12,3xx");
    assert_eq!(obscure_price_digits("-12,345.7"), "-12,3xx");
    assert_eq!(obscure_price_digits("1,234.5"), "1,2xx");
}

#[test]
fn price_privacy_scales_across_mid_price_denominations() {
    assert_eq!(obscure_price_digits("825.42"), "82x");
    assert_eq!(obscure_price_digits("82.54"), "8x");
    assert_eq!(obscure_price_digits("8.254"), "8.xxx");
    assert_eq!(obscure_price_digits("8"), "x");
}

#[test]
fn price_privacy_keeps_only_early_significant_sub_dollar_digits() {
    assert_eq!(obscure_price_digits("0.123456"), "0.1xxxxx");
    assert_eq!(obscure_price_digits("0.012345"), "0.01xxxx");
    assert_eq!(obscure_price_digits("0.00001234"), "0.00001xxx");
    assert_eq!(obscure_price_digits("0.0000"), "0.00xx");
}
