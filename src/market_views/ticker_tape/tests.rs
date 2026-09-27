use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use iced::{Color, Theme};

use super::components::{ticker_tape_bar_style, ticker_tape_divider_style};
use super::formatting::{
    TickerTapeItem, exchange_stat_usd_label, percent_change, percent_label, price_label,
    ticker_tape_item_width,
};
use super::track::ticker_tape_segment_origins;
use super::{TICKER_TAPE_ITEM_MAX_WIDTH, TICKER_TAPE_ITEM_MIN_WIDTH};

// ---------------------------------------------------------------------------
// Ticker Tape Formatting Tests
// ---------------------------------------------------------------------------

#[test]
fn pane_dividers_hide_ticker_tape_border_and_separators_without_changing_geometry() {
    for theme in [Theme::Dark, Theme::Light] {
        let visible = ticker_tape_bar_style(&theme, 8.0, true);
        let hidden = ticker_tape_bar_style(&theme, 8.0, false);

        assert!(visible.border.color.a > 0.0);
        assert_eq!(hidden.border.color, Color::TRANSPARENT);
        assert_eq!(hidden.background, visible.background);
        assert_eq!(hidden.text_color, visible.text_color);
        assert_eq!(hidden.border.width, visible.border.width);
        assert_eq!(hidden.border.radius, visible.border.radius);

        for opacity in [0.10, 0.28] {
            let visible = ticker_tape_divider_style(&theme, opacity, true);
            let hidden = ticker_tape_divider_style(&theme, opacity, false);
            assert_eq!(visible.color.a, opacity);
            assert_eq!(hidden.color, Color::TRANSPARENT);
            assert_eq!(hidden.radius, visible.radius);
            assert_eq!(hidden.fill_mode, visible.fill_mode);
            assert_eq!(hidden.snap, visible.snap);
        }
    }
}

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: Some("YES: Will BTC close green?".to_string()),
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        growth_mode: false,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
            contract: crate::api::OutcomeContract::verified_fixture(),
            venue: None,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

#[test]
fn ticker_tape_outcome_favourite_uses_short_condition_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));
    terminal.favourite_symbols = vec!["#950".to_string()];

    let items = terminal.ticker_tape_items();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].symbol, "#950");
    assert_eq!(items[0].ticker, "Will BTC close green?");
}

#[test]
fn ticker_tape_unloaded_outcome_favourite_uses_cached_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.favourite_symbols = vec!["#950".to_string()];
    terminal
        .outcome_display_labels
        .insert("#950".to_string(), "YES: Will BTC close green?".to_string());

    let items = terminal.ticker_tape_items();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].ticker, "YES: Will BTC close green?");
}

#[test]
fn percent_change_preserves_signed_relative_move() {
    assert_eq!(percent_change(Some(110.0), Some(100.0)), Some(10.0));
    assert_eq!(percent_change(Some(75.0), Some(100.0)), Some(-25.0));
}

#[test]
fn percent_change_requires_positive_current_and_previous_price() {
    assert_eq!(percent_change(None, Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), None), None);
    assert_eq!(percent_change(Some(0.0), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(0.0)), None);
    assert_eq!(percent_change(Some(f64::NAN), Some(100.0)), None);
    assert_eq!(percent_change(Some(100.0), Some(f64::INFINITY)), None);
}

#[test]
fn ticker_tape_labels_use_existing_placeholder_and_sign_conventions() {
    let denomination = DisplayDenominationContext::usd();

    assert_eq!(price_label(None, &denomination), "-");
    assert_eq!(percent_label(None), "-");
    assert_eq!(percent_label(Some(1.234)), "+1.23%");
    assert_eq!(percent_label(Some(-1.234)), "-1.23%");
    assert_eq!(exchange_stat_usd_label(None), "-");
    assert_eq!(exchange_stat_usd_label(Some(4_250_000_000.0)), "$4.25B");
    assert_eq!(exchange_stat_usd_label(Some(f64::NAN)), "-");
}

#[test]
fn ticker_tape_item_width_stays_within_layout_bounds() {
    let denomination = DisplayDenominationContext::usd();
    let narrow_item = TickerTapeItem {
        symbol: "BTC".to_string(),
        ticker: "BTC".to_string(),
        price: None,
        pct_24h: None,
    };
    let wide_item = TickerTapeItem {
        symbol: "VERY-LONG-SYMBOL-NAME".to_string(),
        ticker: "VERY-LONG-SYMBOL-NAME".to_string(),
        price: Some(123_456.789),
        pct_24h: Some(123.456),
    };

    assert_eq!(
        ticker_tape_item_width(&narrow_item, &denomination),
        TICKER_TAPE_ITEM_MIN_WIDTH
    );
    assert_eq!(
        ticker_tape_item_width(&wide_item, &denomination),
        TICKER_TAPE_ITEM_MAX_WIDTH
    );
}

#[test]
fn ticker_tape_repeated_sequence_has_no_gap_at_cycle_boundary() {
    let sequence_widths = [149.0, 173.0, 201.0];
    let sequence_width = sequence_widths.iter().sum::<f32>();
    let repeated_widths: Vec<f32> = sequence_widths
        .iter()
        .copied()
        .cycle()
        .take(sequence_widths.len() * 2)
        .collect();

    for offset in [0.0, 1.0, sequence_width / 2.0, sequence_width - 0.1] {
        let origins = ticker_tape_segment_origins(&repeated_widths, offset);

        for index in 1..origins.len() {
            let previous_end = origins[index - 1] + repeated_widths[index - 1];
            assert!((origins[index] - previous_end).abs() < f32::EPSILON);
        }

        assert!((origins[sequence_widths.len()] - (sequence_width - offset)).abs() < f32::EPSILON);
        assert!(origins[0] <= 0.0);
        assert!(
            origins.last().expect("last repeated segment")
                + repeated_widths.last().expect("last repeated width")
                >= sequence_width
        );
    }
}
