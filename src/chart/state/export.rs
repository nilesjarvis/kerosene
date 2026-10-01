use super::ChartState;

impl ChartState {
    /// Freeze the actual canvas view without carrying pointer/drag interactions
    /// into an image. In particular, retain the epoch: drawing a snapshot from a
    /// reset chart must use the same pending reset as the live canvas.
    pub(crate) fn snapshot_for_export(&self) -> Self {
        Self {
            scroll_offset: self.scroll_offset,
            candle_width: self.candle_width,
            y_auto: self.y_auto,
            y_offset: self.y_offset,
            y_scale: self.y_scale,
            funding_y_scale: self.funding_y_scale,
            funding_y_offset: self.funding_y_offset,
            hud_follow_price: self.hud_follow_price,
            reset_epoch_seen: self.reset_epoch_seen,
            ..Self::default()
        }
    }
}
