use super::*;
use crate::chart_state::PriceFlashDirection;

#[test]
fn ws_candle_price_flash_changes_only_for_an_applied_price_change() {
    let cases = [
        (
            Candle {
                close: f64::NAN,
                ..candle(7_200_000, 105.0)
            },
            false,
            None,
        ),
        (candle(3_600_000, 105.0), false, None),
        (candle(7_200_000, 100.0), true, None),
        (
            candle(7_200_000, 105.0),
            true,
            Some(PriceFlashDirection::Up),
        ),
        (
            candle(10_800_000, 95.0),
            true,
            Some(PriceFlashDirection::Down),
        ),
    ];
    for (incoming, applied, direction) in cases {
        let mut terminal = TradingTerminal::boot().0;
        terminal.charts.clear();
        let mut instance = ChartInstance::new(1, "BTC".into(), Timeframe::H1);
        instance.chart.set_candles(vec![candle(7_200_000, 100.0)]);
        instance.track_last_price_update(Some(101.0), 100.0, 42);
        let previous_flash = instance.last_price_flash;
        terminal.charts.insert(1, instance);

        let _task = terminal.apply_chart_ws_candle_update(
            1,
            "BTC".into(),
            "1h".into(),
            source_context(&terminal, None),
            incoming.clone(),
        );

        let instance = terminal.charts.get(&1).expect("chart");
        assert_eq!(
            last_close(&terminal, 1),
            Some(if applied { incoming.close } else { 100.0 })
        );
        assert_eq!(instance.candle_ws_updated_at_ms.is_some(), applied);
        assert!(matches!(instance.chart.status, ChartStatus::Loaded));
        if let Some(direction) = direction {
            let flash = instance.last_price_flash.expect("new price flash");
            assert_eq!(flash.direction, direction);
            assert_eq!(flash.previous_close, 100.0);
            assert_eq!(Some(flash.started_at_ms), instance.candle_ws_updated_at_ms);
        } else {
            assert_eq!(instance.last_price_flash, previous_flash);
        }
    }
}
