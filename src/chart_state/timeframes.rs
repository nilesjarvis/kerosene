use crate::app_state::TradingTerminal;
use crate::timeframe::Timeframe;

impl TradingTerminal {
    pub(crate) fn has_open_timeframe_charts(&self) -> bool {
        !self.charts.is_empty() || !self.spaghetti_charts.is_empty()
    }

    /// Derived from current instances, so individual edits and layout switches
    /// immediately update the global picker without a separate saved setting.
    pub(crate) fn common_chart_timeframe(&self) -> Option<Timeframe> {
        let mut timeframes = self.charts.values().map(|chart| chart.interval).chain(
            self.spaghetti_charts.values().map(|chart| {
                Self::spaghetti_effective_timeframe_for(
                    chart.interval,
                    chart.canvas.active_session,
                    chart.session_granularity,
                    self.status_bar_now_ms,
                )
            }),
        );
        let first = timeframes.next()?;
        timeframes
            .all(|timeframe| timeframe == first)
            .then_some(first)
    }
}
