use crate::api;
use crate::app_state::TradingTerminal;
use crate::app_time::now_ms;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{SpaghettiCandleFetch, SpaghettiChartId};
use crate::timeframe::Timeframe;
use iced::Task;

impl TradingTerminal {
    /// Build a Task that fetches candles for a spaghetti chart series.
    pub(crate) fn fetch_spaghetti_candles(
        spaghetti_id: SpaghettiChartId,
        coin: &str,
        tf: Timeframe,
        session: Option<spaghetti::Session>,
        session_granularity: Option<Timeframe>,
        cached_start_ms: Option<u64>,
        backfill: ChartBackfillFetchContext,
    ) -> Task<Message> {
        let now_ms = now_ms();
        let (api_tf, mut start) =
            Self::spaghetti_fetch_plan(tf, session, session_granularity, now_ms);
        if let Some(c) = cached_start_ms
            && c > start
        {
            start = c;
        }
        let sid = spaghetti_id;
        let coin_str = coin.to_string();
        let request = SpaghettiCandleFetch {
            chart_id: sid,
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
                coin_str.clone(),
                api_tf.api_str().to_string(),
                start,
                now_ms,
            ),
            move |result| Message::SpaghettiCandlesLoaded(request.clone(), result),
        )
    }
}
