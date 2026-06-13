use super::{CandleFetchRequest, ChartBackfillFetchContext, ChartBackfillRequestContext, ChartId};
use crate::api::{self, Candle};
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::Task;
use zeroize::Zeroizing;

mod cache;

use self::cache::{get_fresh_cached_candles, store_normalized_candles};

pub(crate) const CANDLE_FETCH_MAX_ATTEMPTS: u8 = 4;

impl TradingTerminal {
    /// Fetch daily/weekly/monthly candles for macro indicators, tagged with
    /// the chart ID and symbol that requested them.
    pub(crate) fn fetch_macro_candles_tasks(
        chart_id: ChartId,
        request_id: u64,
        coin: &str,
    ) -> Vec<Task<Message>> {
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
                move |result| {
                    Message::MacroCandlesLoaded(id, request_id, s1.clone(), Timeframe::D1, result)
                },
            ),
            Task::perform(
                api::fetch_candles(
                    c2,
                    "1w".to_string(),
                    now_ms.saturating_sub(Timeframe::W1.lookback_ms()),
                    now_ms,
                ),
                move |result| {
                    Message::MacroCandlesLoaded(id, request_id, s2.clone(), Timeframe::W1, result)
                },
            ),
            Task::perform(
                api::fetch_candles(
                    c3,
                    "1M".to_string(),
                    now_ms.saturating_sub(Timeframe::Mo1.lookback_ms()),
                    now_ms,
                ),
                move |result| {
                    Message::MacroCandlesLoaded(id, request_id, s3.clone(), Timeframe::Mo1, result)
                },
            ),
        ]
    }

    pub(crate) fn queue_macro_candles_tasks(
        &mut self,
        chart_id: ChartId,
        coin: &str,
    ) -> Vec<Task<Message>> {
        let Some(request_id) = self
            .charts
            .get_mut(&chart_id)
            .map(|instance| instance.next_macro_candles_request_id())
        else {
            return Vec::new();
        };
        Self::fetch_macro_candles_tasks(chart_id, request_id, coin)
    }

    pub(crate) fn build_candle_fetch_request(
        chart_id: ChartId,
        coin: &str,
        tf: Timeframe,
        backfill: ChartBackfillRequestContext,
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
            source: backfill.source,
            read_data_provider_generation: backfill.read_data_provider_generation,
            hydromancer_key_generation: backfill.hydromancer_key_generation,
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

    pub(crate) fn chart_backfill_request_context(&self) -> ChartBackfillRequestContext {
        ChartBackfillRequestContext::new(
            self.chart_backfill_source,
            self.read_data_provider_generation,
            self.hydromancer_key_generation,
        )
    }

    pub(crate) fn fetch_candles_task(
        request: CandleFetchRequest,
        hydromancer_api_key: Zeroizing<String>,
    ) -> Task<Message> {
        let delay_ms = Self::candle_fetch_retry_delay_ms(request.attempt);
        let fetch_request = request.clone();
        Task::perform(
            async move {
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                api::fetch_chart_backfill_candles(
                    fetch_request.source,
                    hydromancer_api_key,
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
        Self::fetch_candles_task(request, self.hydromancer_api_key_for_task())
    }

    pub(crate) fn queue_candle_fetch_for(
        &mut self,
        chart_id: ChartId,
        coin: &str,
        tf: Timeframe,
        cached_start_ms: Option<u64>,
    ) -> Task<Message> {
        let request = Self::build_candle_fetch_request(
            chart_id,
            coin,
            tf,
            self.chart_backfill_request_context(),
            cached_start_ms,
            0,
        );
        self.queue_candle_fetch(request)
    }

    pub(crate) fn reload_chart_backfills_for_source_change(&mut self) -> Task<Message> {
        self.candle_data_cache.clear();
        self.candle_data_cache_order.clear();

        let source = self.chart_backfill_source;
        let backfill_context = self.chart_backfill_request_context();
        let hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_key = self.hydromancer_api_key_for_task();
        let chart_requests: Vec<_> = self
            .charts
            .iter()
            .filter(|(_, instance)| {
                !instance.symbol.is_empty() && !self.symbol_key_is_hidden(&instance.symbol)
            })
            .map(|(chart_id, instance)| {
                Self::build_candle_fetch_request(
                    *chart_id,
                    &instance.symbol,
                    instance.interval,
                    backfill_context,
                    None,
                    0,
                )
            })
            .collect();

        for request in &chart_requests {
            self.clear_chart_heatmap_pending_request_state(request.chart_id);
            self.clear_chart_liquidation_pending_request_state(request.chart_id);
            if let Some(instance) = self.charts.get_mut(&request.chart_id) {
                instance.chart.candles.clear();
                instance.chart.status = ChartStatus::Loading;
                instance.chart.candle_cache.clear();
                instance.candle_fetch_request = Some(request.clone());
                instance.candle_fetch_error = None;
                instance.heatmap_last_fetch = None;
                instance.heatmap_viewport = None;
                instance.heatmap_status = None;
                instance.heatmap_fetching = false;
                instance.last_price_flash = None;
                Self::clear_heatmap_display(instance);
                Self::clear_liquidation_display(instance);
                Self::clear_funding_display(instance);
            }
        }

        let mut tasks: Vec<Task<Message>> = chart_requests
            .into_iter()
            .map(|request| Self::fetch_candles_task(request, hydromancer_key.clone()))
            .collect();

        let spaghetti_requests: Vec<_> = self
            .spaghetti_charts
            .iter()
            .flat_map(|(chart_id, instance)| {
                instance.canvas.series.iter().map(|series| {
                    (
                        *chart_id,
                        series.symbol.clone(),
                        instance.interval,
                        instance.canvas.active_session,
                        instance.session_granularity,
                    )
                })
            })
            .collect();

        for (chart_id, _, _, _, _) in &spaghetti_requests {
            if let Some(instance) = self.spaghetti_charts.get_mut(chart_id) {
                for series in &mut instance.canvas.series {
                    series.candles.clear();
                    series.loaded = false;
                }
                instance.canvas.cache.clear();
            }
        }

        tasks.extend(spaghetti_requests.into_iter().map(
            |(chart_id, symbol, timeframe, session, session_granularity)| {
                Self::fetch_spaghetti_candles(
                    chart_id,
                    &symbol,
                    timeframe,
                    session,
                    session_granularity,
                    None,
                    ChartBackfillFetchContext::new(
                        source,
                        backfill_context.read_data_provider_generation,
                        hydromancer_generation,
                        hydromancer_key.clone(),
                    ),
                )
            },
        ));

        Task::batch(tasks)
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
