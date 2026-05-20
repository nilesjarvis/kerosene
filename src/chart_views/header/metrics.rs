mod columns;

pub(super) use self::columns::ChartHeaderMetricVisibility;
use self::columns::{push_perp_metric_columns, push_spot_metric_columns};

use crate::account::AssetContext;
use crate::chart_state::ChartId;
use crate::denomination::DisplayDenominationContext;
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
    chart_price: f64,
    open_interest_as_notional: bool,
    visibility: ChartHeaderMetricVisibility,
    denomination: &DisplayDenominationContext,
) -> Row<'a, Message> {
    if ctx.funding.is_some() {
        push_perp_metric_columns(
            header_row,
            theme,
            chart_id,
            ctx,
            chart_price,
            open_interest_as_notional,
            visibility,
            denomination,
        )
    } else {
        push_spot_metric_columns(header_row, theme, ctx, denomination)
    }
}
