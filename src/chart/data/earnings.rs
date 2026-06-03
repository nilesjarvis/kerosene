use super::super::{CandlestickChart, EarningsMarker};

// ---------------------------------------------------------------------------
// Earnings Marker Data
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(crate) fn set_earnings_markers(&mut self, markers: Vec<EarningsMarker>) {
        self.earnings_markers = markers;
        if self.hover_earnings_marker_time_ms.is_some_and(|time_ms| {
            !self
                .earnings_markers
                .iter()
                .any(|marker| marker.time_ms == time_ms)
        }) {
            self.hover_earnings_marker_time_ms = None;
            self.earnings_marker_hover_progress = 0.0;
        }
        self.candle_cache.clear();
    }

    pub(crate) fn clear_earnings_markers(&mut self) {
        let had_markers = !self.earnings_markers.is_empty();
        self.hover_earnings_marker_time_ms = None;
        self.earnings_marker_hover_progress = 0.0;
        if had_markers {
            self.earnings_markers.clear();
            self.candle_cache.clear();
        }
    }
}
