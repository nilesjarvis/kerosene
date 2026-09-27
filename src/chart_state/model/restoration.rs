use super::ChartInstance;
use crate::annotations::Annotation;
use crate::config::{ChartConfig, MAX_QUICK_TRADE_ACTIONS};
use crate::timeframe::Timeframe;

impl ChartInstance {
    /// Restore persisted settings with the caller's resolved primary symbol.
    /// Secondary symbols and data requests depend on the available market
    /// metadata and remain the caller's responsibility.
    pub(crate) fn from_config(config: &ChartConfig, symbol: String) -> Self {
        let mut instance = Self::new(
            config.id,
            symbol,
            Timeframe::from_config_str(&config.timeframe),
        );
        instance.chart.inverted = config.inverted;
        instance.chart.show_trade_markers = config.show_trade_markers;
        instance.show_earnings_markers = config.show_earnings_markers;
        instance.header_collapsed = config.header_collapsed;
        instance.drawing_toolbar_collapsed = config.drawing_toolbar_collapsed;
        instance
            .chart
            .set_funding_panel_height(config.funding_panel_height as f32);
        instance
            .chart
            .set_session_panel_height(config.session_panel_height as f32);
        instance.macro_indicators = config.macro_indicators.clone();
        instance.chart.macro_indicators = config.macro_indicators.clone();
        instance.quick_trade_actions = config
            .quick_trade_actions
            .iter()
            .filter(|action| action.is_valid())
            .take(MAX_QUICK_TRADE_ACTIONS)
            .cloned()
            .collect();
        instance.open_interest_as_notional = config.open_interest_as_notional;
        instance.asset_volume_as_notional = config.asset_volume_as_notional;
        instance.outcome_volume_as_notional = config.outcome_volume_as_notional;

        for annotation in &config.annotations {
            if let Some(annotation) =
                Annotation::from_config(instance.next_annotation_id, annotation)
            {
                instance.annotations.push(annotation);
                instance.next_annotation_id += 1;
            }
        }
        instance.chart.annotations = instance.annotations.clone();
        instance
    }
}
