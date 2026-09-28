use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use crate::market_state::{LiveWatchlistId, LiveWatchlistInstance};
use std::collections::{HashMap, HashSet};

impl TradingTerminal {
    pub(super) fn boot_live_watchlists(
        cfg: &KeroseneConfig,
        muted_tickers: &HashSet<String>,
    ) -> HashMap<LiveWatchlistId, LiveWatchlistInstance> {
        cfg.live_watchlists
            .clone()
            .into_iter()
            .map(|watchlist_config| {
                (
                    watchlist_config.id,
                    LiveWatchlistInstance {
                        id: watchlist_config.id,
                        preset_id: watchlist_config.preset_id,
                        symbols: watchlist_config
                            .preset_id
                            .and_then(|preset_id| {
                                cfg.watchlist_presets
                                    .iter()
                                    .find(|preset| preset.id == preset_id)
                                    .map(|preset| preset.symbols.clone())
                            })
                            .unwrap_or(watchlist_config.symbols)
                            .into_iter()
                            .filter(|symbol| {
                                !Self::key_matches_muted_tickers(&[], muted_tickers, symbol)
                            })
                            .collect(),
                        search_query: String::new(),
                        sort_column: watchlist_config.sort_column,
                        sort_direction: watchlist_config.sort_direction,
                        visible_columns: watchlist_config.visible_columns,
                        ema_period_input: watchlist_config.ema.period.to_string(),
                        ema: watchlist_config.ema,
                        row_cache: Vec::new(),
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_watchlist_ema_boot_preserves_saved_settings() {
        let config: KeroseneConfig = serde_json::from_value(serde_json::json!({
            "live_watchlists": [{"id": 42, "symbols": ["BTC"],
                "visible_columns": ["EmaDistance"], "ema": {"period": 50, "timeframe": "1d"}}]
        }))
        .expect("config");
        let watchlists = TradingTerminal::boot_live_watchlists(&config, &HashSet::new());
        assert_eq!(watchlists[&42].ema.period, 50);
        assert_eq!(watchlists[&42].ema.timeframe, "1d");
        assert_eq!(watchlists[&42].ema_period_input, "50");
    }
}
