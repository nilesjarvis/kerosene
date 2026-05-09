use super::{CandleFetchRequest, ChartId};
use crate::api::{self, Candle};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::Task;

mod cache;

use self::cache::{get_fresh_cached_candles, store_normalized_candles};

pub(crate) const CANDLE_FETCH_MAX_ATTEMPTS: u8 = 4;

impl TradingTerminal {
    /// Fetch daily/weekly/monthly candles for macro indicators, tagged with
    /// the chart ID and symbol that requested them.
    pub(crate) fn fetch_macro_candles_tasks(chart_id: ChartId, coin: &str) -> Vec<Task<Message>> {
        let now_ms = Self::now_ms();

        let id = chart_id;
        let c1 = coin.to_string();
        let c2 = coin.to_string();
        let c3 = coin.to_string();
        let s1 = c1.clone();
        let s2 = c2.clone();
        let s3 = c3.clone();

        vec![
            Task::perform(
                api::fetch_candles(
                    c1,
                    "1d".to_string(),
                    now_ms.saturating_sub(Timeframe::D1.lookback_ms()),
                    now_ms,
                ),
                move |result| Message::MacroCandlesLoaded(id, s1.clone(), Timeframe::D1, result),
            ),
            Task::perform(
                api::fetch_candles(
                    c2,
                    "1w".to_string(),
                    now_ms.saturating_sub(Timeframe::W1.lookback_ms()),
                    now_ms,
                ),
                move |result| Message::MacroCandlesLoaded(id, s2.clone(), Timeframe::W1, result),
            ),
            Task::perform(
                api::fetch_candles(
                    c3,
                    "1M".to_string(),
                    now_ms.saturating_sub(Timeframe::Mo1.lookback_ms()),
                    now_ms,
                ),
                move |result| Message::MacroCandlesLoaded(id, s3.clone(), Timeframe::Mo1, result),
            ),
        ]
    }

    pub(crate) fn build_candle_fetch_request(
        chart_id: ChartId,
        coin: &str,
        tf: Timeframe,
        cached_start_ms: Option<u64>,
        attempt: u8,
    ) -> CandleFetchRequest {
        let now_ms = Self::now_ms();
        let start = match cached_start_ms {
            Some(t) => t.saturating_sub(tf.duration_ms().saturating_mul(2)),
            None => now_ms.saturating_sub(tf.lookback_ms()),
        };
        CandleFetchRequest {
            chart_id,
            symbol: coin.to_string(),
            timeframe: tf,
            start_ms: start,
            end_ms: now_ms,
            attempt,
        }
    }

    pub(crate) fn candle_fetch_retry_delay_ms(attempt: u8) -> u64 {
        match attempt {
            0 => 0,
            1 => 1_000,
            2 => 3_000,
            _ => 8_000,
        }
    }

    pub(crate) fn fetch_candles_task(request: CandleFetchRequest) -> Task<Message> {
        let delay_ms = Self::candle_fetch_retry_delay_ms(request.attempt);
        let fetch_request = request.clone();
        Task::perform(
            async move {
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                api::fetch_candles(
                    fetch_request.symbol,
                    fetch_request.timeframe.api_str().to_string(),
                    fetch_request.start_ms,
                    fetch_request.end_ms,
                )
                .await
            },
            move |result| Message::ChartCandlesLoaded(request.clone(), result),
        )
    }

    pub(crate) fn queue_candle_fetch(&mut self, request: CandleFetchRequest) -> Task<Message> {
        if let Some(instance) = self.charts.get_mut(&request.chart_id) {
            instance.candle_fetch_request = Some(request.clone());
            instance.candle_fetch_error = None;
            if instance.chart.candles.is_empty() {
                instance.chart.status = ChartStatus::Loading;
            }
        }
        Self::fetch_candles_task(request)
    }

    pub(crate) fn queue_candle_fetch_for(
        &mut self,
        chart_id: ChartId,
        coin: &str,
        tf: Timeframe,
        cached_start_ms: Option<u64>,
    ) -> Task<Message> {
        let request = Self::build_candle_fetch_request(chart_id, coin, tf, cached_start_ms, 0);
        self.queue_candle_fetch(request)
    }

    pub(crate) fn cache_candles(&mut self, symbol: &str, tf: Timeframe, candles: Vec<Candle>) {
        store_normalized_candles(
            &mut self.candle_data_cache,
            &mut self.candle_data_cache_order,
            symbol,
            tf,
            candles,
        );
    }

    pub(crate) fn get_cached_candles(
        &mut self,
        symbol: &str,
        tf: Timeframe,
    ) -> Option<Vec<Candle>> {
        get_fresh_cached_candles(
            &mut self.candle_data_cache,
            &mut self.candle_data_cache_order,
            symbol,
            tf,
            Self::now_ms(),
        )
    }
}
