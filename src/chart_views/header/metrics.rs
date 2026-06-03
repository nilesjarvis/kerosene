mod columns;

pub(super) use self::columns::{ChartHeaderMetricVisibility, push_outcome_volume_column};
use self::columns::{push_perp_metric_columns, push_spot_metric_columns};

use crate::account::AssetContext;
use crate::chart_state::ChartId;
use crate::message::Message;

use iced::Theme;
use iced::widget::Row;

// ---------------------------------------------------------------------------
// Header Metrics
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(super) fn push_asset_context_columns<'a>(
    header_row: Row<'a, Message>,
    theme: &Theme,
    chart_id: ChartId,
    ctx: &'a AssetContext,
    symbol_display: &str,
    chart_price: f64,
    asset_volume_as_notional: bool,
    open_interest_as_notional: bool,
    visibility: ChartHeaderMetricVisibility,
) -> Row<'a, Message> {
    if ctx.funding.is_some() {
        push_perp_metric_columns(
            header_row,
            theme,
            chart_id,
            ctx,
            symbol_display,
            chart_price,
            asset_volume_as_notional,
            open_interest_as_notional,
            visibility,
        )
    } else {
        push_spot_metric_columns(
            header_row,
            theme,
            chart_id,
            ctx,
            symbol_display,
            asset_volume_as_notional,
        )
    }
}
