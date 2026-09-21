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
        instance_epoch: u64,
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
        static NEXT_REQUEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let request = SpaghettiCandleFetch {
            request_id: NEXT_REQUEST.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            start_ms: start,
            end_ms: now_ms,
            chart_id: sid,
            instance_epoch,
            symbol: coin_str.clone(),
            timeframe: api_tf,
            source: backfill.source,
            read_data_provider_generation: backfill.read_data_provider_generation,
            hydromancer_key_generation: backfill.hydromancer_key_generation,
            session,
            session_granularity,
        };
        drop(backfill.hydromancer_api_key);
        Task::done(Message::SpaghettiFetchRequested(request))
    }
}
