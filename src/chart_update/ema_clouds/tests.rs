use super::*;
use crate::chart_state::ChartInstance;
use crate::config::EmaCloudColor;
use crate::timeframe::Timeframe;

fn terminal_with_cloud() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    for id in [7, 8] {
        terminal
            .charts
            .insert(id, ChartInstance::new(id, "BTC".into(), Timeframe::H1));
    }
    let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudAdded(7));
    terminal
}

#[test]
fn ema_cloud_edits_are_per_chart_and_survive_snapshots_restoration_and_detaching() {
    let mut terminal = terminal_with_cloud();
    for message in [
        Message::ChartEmaCloudPeriodChanged(7, 1, EmaCloudPeriod::Fast, "9".into()),
        Message::ChartEmaCloudPeriodChanged(7, 1, EmaCloudPeriod::Slow, "21".into()),
        Message::ChartEmaCloudTimeframeChanged(7, 1, EmaCloudTimeframe::Day),
        Message::ChartEmaCloudColorChanged(7, 1, EmaCloudColor::Secondary),
        Message::ChartEmaCloudOpacityChanged(7, 1, 35),
    ] {
        let _task = terminal.update_chart_ema_clouds(message);
    }
    let instance = &terminal.charts[&7];
    let cloud = &instance.macro_indicators.ema_clouds[0];
    assert_eq!(
        (cloud.fast_period, cloud.slow_period, cloud.opacity),
        (9, 21, 35)
    );
    assert_eq!(cloud.timeframe, EmaCloudTimeframe::Day);
    assert_eq!(cloud.color, EmaCloudColor::Secondary);
    assert_eq!(
        instance.chart.macro_indicators.ema_clouds,
        vec![cloud.clone()]
    );
    assert!(terminal.charts[&8].macro_indicators.ema_clouds.is_empty());
    let detached = instance.clone_for_detached_window(9);
    assert_eq!(
        detached.chart.macro_indicators.ema_clouds,
        vec![cloud.clone()]
    );
    assert!(detached.ema_cloud_period_inputs.is_empty());
    let snapshot = terminal
        .chart_configs_snapshot()
        .into_iter()
        .find(|config| config.id == 7)
        .expect("chart snapshot");
    let restored = ChartInstance::from_config(&snapshot, snapshot.symbol.clone());
    assert_eq!(
        restored.chart.macro_indicators.ema_clouds,
        vec![cloud.clone()]
    );
    assert!(terminal.config_save_due_at.is_some());
}

#[test]
fn ema_cloud_invalid_periods_keep_last_valid_value_and_menu_reopening_resets_drafts() {
    let mut terminal = terminal_with_cloud();
    for value in ["", "0", "-1", "abc", "1.5", "5001", "9999999999999999999"] {
        let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudPeriodChanged(
            7,
            1,
            EmaCloudPeriod::Fast,
            value.into(),
        ));
        assert_eq!(
            terminal.charts[&7].chart.macro_indicators.ema_clouds[0].fast_period,
            20
        );
    }
    let _task = terminal.update_chart_macro_indicators(Message::ToggleMacroMenu(7));
    assert!(terminal.charts[&7].ema_cloud_period_inputs.is_empty());
}

#[test]
fn ema_cloud_add_toggle_remove_are_bounded_and_preserve_other_clouds() {
    let mut terminal = terminal_with_cloud();
    for _ in 0..MAX_EMA_CLOUDS + 2 {
        let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudAdded(7));
    }
    assert_eq!(
        terminal.charts[&7].macro_indicators.ema_clouds.len(),
        MAX_EMA_CLOUDS
    );
    let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudToggled(7, 1));
    assert!(!terminal.charts[&7].chart.macro_indicators.ema_clouds[0].enabled);
    let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudRemoved(7, 1));
    assert_eq!(
        terminal.charts[&7].chart.macro_indicators.ema_clouds[0].id,
        2
    );
    let before = terminal.charts[&7].chart.macro_indicators.clone();
    let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudPeriodChanged(
        7,
        1,
        EmaCloudPeriod::Slow,
        "99".into(),
    ));
    assert_eq!(terminal.charts[&7].chart.macro_indicators, before);
}

#[test]
fn ema_cloud_larger_period_requests_extra_history_and_disabled_cloud_does_not() {
    let mut terminal = terminal_with_cloud();
    let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudPeriodChanged(
        7,
        1,
        EmaCloudPeriod::Slow,
        "1000".into(),
    ));
    let request = terminal.charts[&7]
        .candle_fetch_request
        .as_ref()
        .expect("history request");
    assert_eq!(
        request.end_ms - request.start_ms,
        Timeframe::H1.lookback_ms() + 3_000 * Timeframe::H1.duration_ms()
    );
    let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudToggled(7, 1));
    terminal
        .charts
        .get_mut(&7)
        .expect("chart")
        .candle_fetch_request = None;
    let _task = terminal.update_chart_ema_clouds(Message::ChartEmaCloudPeriodChanged(
        7,
        1,
        EmaCloudPeriod::Slow,
        "2000".into(),
    ));
    assert!(terminal.charts[&7].candle_fetch_request.is_none());
}
