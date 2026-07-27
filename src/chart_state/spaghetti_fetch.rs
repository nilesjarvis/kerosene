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
        backfill: ChartBackfillFetchContext,
    ) -> Task<Message> {
        let now_ms = now_ms();
        let (api_tf, start) = Self::spaghetti_fetch_plan(tf, session, session_granularity, now_ms);
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
            api::fetch_chart_backfill_candles(api::ChartCandleFetchRequest {
                source: backfill.source,
                hydromancer_api_key: backfill.hydromancer_api_key,
                schwab_access_token: zeroize::Zeroizing::new(String::new()),
                coin: coin_str.clone(),
                interval: api_tf.api_str().to_string(),
                start_time: start,
                end_time: now_ms,
                policy: api::CandleFetchPolicy::NetworkOnly,
            }),
            move |result| Message::SpaghettiCandlesLoaded(request.clone(), result),
        )
    }
}
