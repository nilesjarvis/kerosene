use crate::api;
use crate::app_state::TradingTerminal;
use crate::app_time::now_ms;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiCandleFetch, SpaghettiChartInstance};
use iced::Task;

impl TradingTerminal {
    /// Build a Task that fetches candles for a spaghetti chart series.
    pub(crate) fn queue_spaghetti_candle_fetch(
        instance: &mut SpaghettiChartInstance,
        coin: &str,
        chart_instance_generation: u64,
        cached_start_ms: Option<u64>,
        backfill: ChartBackfillFetchContext,
    ) -> Task<Message> {
        let now_ms = now_ms();
        let session = instance.canvas.active_session;
        let session_granularity = instance.session_granularity;
        let (api_tf, mut start) =
            Self::spaghetti_fetch_plan(instance.interval, session, session_granularity, now_ms);
        if let Some(c) = cached_start_ms
            && c > start
        {
            start = c;
        }
        let Some(request_id) = instance.begin_spaghetti_candle_request(coin) else {
            return Task::none();
        };
        let chart_id = instance.id;
        let coin_str = coin.to_string();
        let request = SpaghettiCandleFetch {
            chart_id,
            chart_instance_generation,
            request_id,
            symbol: coin_str.clone(),
            timeframe: api_tf,
            source: backfill.source,
            read_data_provider_generation: backfill.read_data_provider_generation,
            hydromancer_key_generation: backfill.hydromancer_key_generation,
            session,
            session_granularity,
        };
        Task::perform(
            api::fetch_chart_backfill_candles(
                backfill.source,
                backfill.hydromancer_api_key,
                zeroize::Zeroizing::new(String::new()),
                coin_str.clone(),
                api_tf.api_str().to_string(),
                start,
                now_ms,
            ),
            move |result| Message::SpaghettiCandlesLoaded(request.clone(), result.into()),
        )
    }
}
