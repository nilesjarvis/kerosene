use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::SpaghettiChartId;
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
    ) -> Task<Message> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let (api_tf, mut start) =
            Self::spaghetti_fetch_plan(tf, session, session_granularity, now_ms);
        if let Some(c) = cached_start_ms
            && c > start
        {
            start = c;
        }
        let sid = spaghetti_id;
        let coin_str = coin.to_string();
        Task::perform(
            api::fetch_candles(
                coin_str.clone(),
                api_tf.api_str().to_string(),
                start,
                now_ms,
            ),
            move |result| Message::SpaghettiCandlesLoaded(sid, coin_str.clone(), result),
        )
    }
}
