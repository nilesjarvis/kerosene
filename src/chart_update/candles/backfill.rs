use crate::app_state::TradingTerminal;
use crate::chart::{CANDLE_GAP_RATIO, ChartViewport, MAX_CHART_CANDLES};
use crate::chart_state::{CandleFetchRequest, ChartId};
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(in crate::chart_update) fn maybe_backfill_chart_candles_for_viewport(
        &mut self,
        chart_id: ChartId,
        viewport: ChartViewport,
    ) -> Task<Message> {
        let primary_request = self.plan_older_primary_candle_fetch(chart_id, viewport);
        let secondary_request = self.plan_older_secondary_candle_fetch(chart_id, viewport);

        let mut tasks = Vec::new();
        if let Some(request) = primary_request {
            tasks.push(self.queue_candle_fetch(request));
        }
        if let Some(request) = secondary_request {
            tasks.push(self.queue_secondary_candle_fetch(request));
        }

        Task::batch(tasks)
    }

    pub(in crate::chart_update) fn maybe_continue_chart_candle_backfill(
        &mut self,
        chart_id: ChartId,
    ) -> Task<Message> {
        let Some(viewport) = self
            .charts
            .get(&chart_id)
            .and_then(|instance| instance.heatmap_viewport)
        else {
            return Task::none();
        };

        self.maybe_backfill_chart_candles_for_viewport(chart_id, viewport)
    }

    pub(in crate::chart_update) fn continue_older_primary_candle_backfill(
        &mut self,
        chart_id: ChartId,
    ) -> Task<Message> {
        match self.plan_next_older_primary_candle_fetch(chart_id) {
            Some(request) => self.queue_candle_fetch(request),
            None => Task::none(),
        }
    }

    pub(in crate::chart_update) fn continue_older_secondary_candle_backfill(
        &mut self,
        chart_id: ChartId,
    ) -> Task<Message> {
        match self.plan_next_older_secondary_candle_fetch(chart_id) {
            Some(request) => self.queue_secondary_candle_fetch(request),
            None => Task::none(),
        }
    }

    fn plan_older_primary_candle_fetch(
        &self,
        chart_id: ChartId,
        viewport: ChartViewport,
    ) -> Option<CandleFetchRequest> {
        // Only the viewport-edge gate lives here; `plan_next_older_*` owns the
        // shared readiness guard so the two entry points cannot drift.
        let instance = self.charts.get(&chart_id)?;
        let candles = instance.chart.candles.as_slice();
        let oldest_loaded_open_ms = candles.first()?.open_time;
        if !viewport_reaches_oldest_loaded(
            viewport,
            candles.len(),
            oldest_loaded_open_ms,
            instance.interval.duration_ms(),
        ) {
            return None;
        }

        self.plan_next_older_primary_candle_fetch(chart_id)
    }

    fn plan_next_older_primary_candle_fetch(
        &self,
        chart_id: ChartId,
    ) -> Option<CandleFetchRequest> {
        let instance = self.charts.get(&chart_id)?;
        let candle_count = instance.chart.candles.len();
        if instance.symbol.is_empty()
            || self.symbol_key_is_hidden(&instance.symbol)
            || !instance.interval.uses_candle_backfill()
            || instance.candle_fetch_request.is_some()
            || instance.candle_backfill_exhausted
            || candle_count >= MAX_CHART_CANDLES
        {
            return None;
        }

        let oldest_loaded_open_ms = instance.chart.candles.first()?.open_time;
        Self::build_older_candle_fetch_request(
            chart_id,
            &instance.symbol,
            instance.interval,
            self.chart_backfill_request_context_for_symbol_timeframe(
                &instance.symbol,
                instance.interval,
            ),
            oldest_loaded_open_ms,
            MAX_CHART_CANDLES - candle_count,
        )
    }

    fn plan_older_secondary_candle_fetch(
        &self,
        chart_id: ChartId,
        viewport: ChartViewport,
    ) -> Option<CandleFetchRequest> {
        let instance = self.charts.get(&chart_id)?;
        let candles = instance.chart.secondary_series.as_ref()?.candles.as_slice();
        let oldest_loaded_open_ms = candles.first()?.open_time;
        if !viewport_reaches_oldest_loaded(
            viewport,
            candles.len(),
            oldest_loaded_open_ms,
            instance.interval.duration_ms(),
        ) {
            return None;
        }

        self.plan_next_older_secondary_candle_fetch(chart_id)
    }

    fn plan_next_older_secondary_candle_fetch(
        &self,
        chart_id: ChartId,
    ) -> Option<CandleFetchRequest> {
        let instance = self.charts.get(&chart_id)?;
        let symbol = instance.secondary_symbol.as_ref()?;
        let candle_count = instance.chart.secondary_series.as_ref()?.candles.len();
        if symbol.is_empty()
            || self.symbol_key_is_hidden(symbol)
            || !instance.interval.uses_candle_backfill()
            || instance.secondary_candle_fetch_request.is_some()
            || instance.secondary_candle_backfill_exhausted
            || candle_count >= MAX_CHART_CANDLES
        {
            return None;
        }

        let oldest_loaded_open_ms = instance
            .chart
            .secondary_series
            .as_ref()?
            .candles
            .first()?
            .open_time;
        Self::build_older_candle_fetch_request(
            chart_id,
            symbol,
            instance.interval,
            self.chart_backfill_request_context_for_symbol_timeframe(symbol, instance.interval),
            oldest_loaded_open_ms,
            MAX_CHART_CANDLES - candle_count,
        )
    }
}

fn viewport_reaches_oldest_loaded(
    viewport: ChartViewport,
    candle_count: usize,
    oldest_loaded_open_ms: u64,
    timeframe_ms: u64,
) -> bool {
    if candle_count == 0 {
        return false;
    }

    let edge_threshold_ms = timeframe_ms;
    if viewport.start_time_ms <= oldest_loaded_open_ms.saturating_add(edge_threshold_ms) {
        return true;
    }

    if viewport.chart_width <= 0.0
        || !viewport.chart_width.is_finite()
        || viewport.candle_width <= 0.0
        || !viewport.candle_width.is_finite()
        || !viewport.scroll_offset.is_finite()
    {
        return false;
    }

    let step = viewport.candle_width * (1.0 + CANDLE_GAP_RATIO);
    if step <= 0.0 || !step.is_finite() {
        return false;
    }

    // Clamp before the `as isize` casts. Candle counts are bounded by
    // `MAX_CHART_CANDLES`, so bounding the visible-slot count and the scroll
    // offset to that range keeps both the conversions and the subtractions
    // below from overflowing on a degenerate (tiny) candle width or an
    // out-of-range scroll offset, without affecting any realistic viewport.
    let max_index = candle_count as f32;
    let visible_slots = (viewport.chart_width / step)
        .ceil()
        .clamp(0.0, max_index + 1.0) as isize
        + 1;
    let scroll_offset = viewport
        .scroll_offset
        .clamp(-max_index - 1.0, max_index + 1.0) as isize;
    let last_idx = candle_count as isize - 1;
    let right_idx = last_idx - scroll_offset;
    let left_idx = right_idx - visible_slots;
    left_idx < 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Candle;
    use crate::chart_state::{CandleFetchMode, ChartInstance};
    use crate::timeframe::Timeframe;

    fn candle(open_time: u64) -> Candle {
        Candle::test_flat(open_time, 100.0)
    }

    fn viewport(start_time_ms: u64) -> ChartViewport {
        ChartViewport {
            start_time_ms,
            end_time_ms: start_time_ms + 60_000,
            price_lo: 90.0,
            price_hi: 110.0,
            chart_width: 400.0,
            candle_width: 10.0,
            scroll_offset: 0.0,
            y_auto: true,
            y_scale: 1.0,
            y_offset: 0.0,
            funding_y_scale: 1.0,
            funding_y_offset: 0.0,
        }
    }

    #[test]
    fn viewport_at_oldest_loaded_candle_queues_older_primary_backfill() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![candle(3_600_000), candle(7_200_000)]);
        terminal.charts.insert(1, instance);

        let _task = terminal.maybe_backfill_chart_candles_for_viewport(1, viewport(3_600_000));

        let request = terminal
            .charts
            .get(&1)
            .and_then(|instance| instance.candle_fetch_request.as_ref())
            .expect("older candle request");
        assert_eq!(request.mode, CandleFetchMode::BackfillOlder);
        assert_eq!(request.symbol, "BTC");
        assert_eq!(request.end_ms, 3_599_999);
        assert_eq!(request.start_ms, 0);
    }

    #[test]
    fn viewport_away_from_oldest_loaded_candle_does_not_backfill() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.chart.set_candles(vec![
            candle(3_600_000),
            candle(7_200_000),
            candle(10_800_000),
        ]);
        terminal.charts.insert(1, instance);

        let mut away_viewport = viewport(10_800_000);
        away_viewport.chart_width = 10.0;

        let _task = terminal.maybe_backfill_chart_candles_for_viewport(1, away_viewport);

        assert!(
            terminal
                .charts
                .get(&1)
                .expect("chart")
                .candle_fetch_request
                .is_none()
        );
    }

    #[test]
    fn exhausted_backfill_boundary_does_not_refetch_empty_oldest_page() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![candle(3_600_000), candle(7_200_000)]);
        instance.candle_backfill_exhausted = true;
        terminal.charts.insert(1, instance);

        let _task = terminal.maybe_backfill_chart_candles_for_viewport(1, viewport(3_600_000));

        assert!(
            terminal
                .charts
                .get(&1)
                .expect("chart")
                .candle_fetch_request
                .is_none()
        );
    }

    #[test]
    fn older_primary_continuation_queues_next_page_without_viewport_edge_check() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.charts.clear();

        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![candle(14_400_000), candle(18_000_000)]);
        terminal.charts.insert(1, instance);

        let _task = terminal.continue_older_primary_candle_backfill(1);

        let request = terminal
            .charts
            .get(&1)
            .and_then(|instance| instance.candle_fetch_request.as_ref())
            .expect("older candle request");
        assert_eq!(request.mode, CandleFetchMode::BackfillOlder);
        assert_eq!(request.end_ms, 14_399_999);
        assert_eq!(request.start_ms, 0);
    }
}
