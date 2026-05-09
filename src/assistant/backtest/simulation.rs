use super::model::DrawdownDcaResult;
use crate::api::Candle;

pub(super) fn simulate_hourly_dca(
    candles: Vec<Candle>,
    symbol: String,
    interval: String,
    lookback_days: u32,
    tranche_usd: f64,
) -> Result<DrawdownDcaResult, String> {
    if candles.is_empty() {
        return Err("No candle data available for selected range".to_string());
    }

    let closes = valid_candle_closes(&candles)?;
    let mut units = 0.0;
    let mut invested = 0.0;
    let entries = closes.len();
    for close in &closes {
        units += tranche_usd / *close;
        invested += tranche_usd;
    }

    let end_price = closes[closes.len() - 1];
    let ending_value = units * end_price;
    let pnl = ending_value - invested;
    let roi = if invested > 0.0 {
        (pnl / invested) * 100.0
    } else {
        0.0
    };

    Ok(DrawdownDcaResult {
        symbol,
        interval,
        lookback_days,
        drawdown_pct: 0.0,
        tranche_usd,
        entries,
        invested_usd: invested,
        units,
        end_price,
        ending_value_usd: ending_value,
        pnl_usd: pnl,
        roi_pct: roi,
    })
}

pub(super) fn simulate_drawdown_dca(
    candles: Vec<Candle>,
    symbol: String,
    interval: String,
    lookback_days: u32,
    tranche_usd: f64,
    drawdown_pct: f64,
) -> Result<DrawdownDcaResult, String> {
    if candles.is_empty() {
        return Err("No candle data available for selected range".to_string());
    }

    let closes = valid_candle_closes(&candles)?;
    let mut peak = closes[0];
    let mut armed = true;
    let threshold = drawdown_pct / 100.0;
    let mut units = 0.0;
    let mut invested = 0.0;
    let mut entries = 0_usize;

    for close in &closes {
        if *close > peak {
            peak = *close;
            armed = true;
        }
        let trigger_px = peak * (1.0 - threshold);
        if armed && *close <= trigger_px {
            units += tranche_usd / *close;
            invested += tranche_usd;
            entries += 1;
            armed = false;
        }
    }

    let end_price = closes[closes.len() - 1];
    let ending_value = units * end_price;
    let pnl = ending_value - invested;
    let roi = if invested > 0.0 {
        (pnl / invested) * 100.0
    } else {
        0.0
    };

    Ok(DrawdownDcaResult {
        symbol,
        interval,
        lookback_days,
        drawdown_pct,
        tranche_usd,
        entries,
        invested_usd: invested,
        units,
        end_price,
        ending_value_usd: ending_value,
        pnl_usd: pnl,
        roi_pct: roi,
    })
}

fn valid_candle_closes(candles: &[Candle]) -> Result<Vec<f64>, String> {
    let mut closes = Vec::with_capacity(candles.len());
    for candle in candles {
        if !candle.close.is_finite() || candle.close <= 0.0 {
            return Err("Candle data contained invalid close prices".to_string());
        }
        closes.push(candle.close);
    }
    Ok(closes)
}

#[cfg(test)]
mod tests {
    use super::{simulate_drawdown_dca, simulate_hourly_dca};
    use crate::api::Candle;

    fn candle(open_time: u64, close: f64) -> Candle {
        Candle {
            open_time,
            close_time: open_time + 60_000,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn hourly_dca_rejects_invalid_close_prices() {
        let result = simulate_hourly_dca(
            vec![candle(1, 100.0), candle(2, f64::NAN)],
            "BTC".to_string(),
            "1h".to_string(),
            7,
            100.0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn drawdown_dca_uses_last_valid_close_as_end_price() {
        let result = simulate_drawdown_dca(
            vec![candle(1, 100.0), candle(2, 90.0), candle(3, 95.0)],
            "BTC".to_string(),
            "1h".to_string(),
            7,
            100.0,
            10.0,
        )
        .expect("backtest result");

        assert_eq!(result.entries, 1);
        assert_eq!(result.end_price, 95.0);
    }
}
