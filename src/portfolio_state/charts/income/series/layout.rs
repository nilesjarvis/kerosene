use iced::Point;

use super::{
    BAR_HEIGHT_RATIO, BOTTOM_PAD, IncomeBarLayout, IncomeChartLayout, LEFT_PAD, MIN_SCALE,
    RIGHT_PAD, TOP_PAD,
};

// ---------------------------------------------------------------------------
// Bar Layout
// ---------------------------------------------------------------------------

pub(in crate::portfolio_state::charts::income) fn prepare_income_chart_layout(
    bars: &[(String, f64)],
    width: f32,
    height: f32,
) -> Option<IncomeChartLayout> {
    if bars.is_empty() {
        return None;
    }

    let max_abs = bars
        .iter()
        .map(|(_, value)| value.abs())
        .fold(0.0_f64, f64::max)
        .max(MIN_SCALE);

    let plot_width = (width - LEFT_PAD - RIGHT_PAD).max(1.0);
    let plot_height = (height - TOP_PAD - BOTTOM_PAD).max(1.0);
    let zero_y = TOP_PAD + plot_height * 0.5;
    let group_width = plot_width / bars.len() as f32;
    let bar_width = (group_width * 0.62).clamp(8.0, 44.0);

    let layouts = bars
        .iter()
        .enumerate()
        .map(|(idx, (label, value))| {
            let center_x = LEFT_PAD + group_width * (idx as f32 + 0.5);
            let scaled = ((*value / max_abs) as f32) * (plot_height * BAR_HEIGHT_RATIO);
            let (y, height) = if scaled >= 0.0 {
                (zero_y - scaled, scaled.max(1.0))
            } else {
                (zero_y, (-scaled).max(1.0))
            };

            IncomeBarLayout {
                label: label.clone(),
                value: *value,
                center_x,
                x: center_x - bar_width * 0.5,
                y,
                width: bar_width,
                height,
                scaled,
                show_axis_label: idx % 2 == 0,
            }
        })
        .collect();

    Some(IncomeChartLayout {
        bars: layouts,
        left_pad: LEFT_PAD,
        top_pad: TOP_PAD,
        bottom_pad: BOTTOM_PAD,
        plot_width,
        plot_height,
        zero_y,
        group_width,
    })
}

pub(in crate::portfolio_state::charts::income) fn hovered_income_bar(
    layout: &IncomeChartLayout,
    cursor: Point,
) -> Option<&IncomeBarLayout> {
    if cursor.x < layout.left_pad
        || cursor.x > layout.left_pad + layout.plot_width
        || cursor.y < layout.top_pad
        || cursor.y > layout.top_pad + layout.plot_height
    {
        return None;
    }

    let raw_idx = ((cursor.x - layout.left_pad) / layout.group_width).floor() as usize;
    let idx = raw_idx.min(layout.bars.len().saturating_sub(1));
    layout.bars.get(idx)
}
