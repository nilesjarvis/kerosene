#[derive(Debug, Clone)]
pub struct WatchlistContext {
    pub funding: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub day_vlm: Option<f64>,
}
