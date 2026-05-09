mod columns;

use self::columns::{push_perp_metric_columns, push_spot_metric_columns};

use crate::account::AssetContext;
use crate::message::Message;

use iced::Theme;
use iced::widget::Row;

// ---------------------------------------------------------------------------
// Header Metrics
// ---------------------------------------------------------------------------

pub(super) fn push_asset_context_columns<'a>(
    header_row: Row<'a, Message>,
    theme: &Theme,
    ctx: &'a AssetContext,
) -> Row<'a, Message> {
    if ctx.funding.is_some() {
        push_perp_metric_columns(header_row, theme, ctx)
    } else {
        push_spot_metric_columns(header_row, theme, ctx)
    }
}
