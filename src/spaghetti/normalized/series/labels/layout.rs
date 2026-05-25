use super::{SERIES_LABEL_GAP, SERIES_LABEL_HEIGHT, SERIES_LABEL_MARGIN};

// ---------------------------------------------------------------------------
// Series Label Layout
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::spaghetti::normalized) struct SeriesLabelAnchor {
    pub(in crate::spaghetti::normalized) index: usize,
    pub(in crate::spaghetti::normalized) y: f32,
}

pub(in crate::spaghetti::normalized) fn stack_series_label_positions(
    mut anchors: Vec<SeriesLabelAnchor>,
    chart_h: f32,
) -> Vec<Option<f32>> {
    if anchors.is_empty() {
        return Vec::new();
    }

    let slot_count = anchors.iter().map(|anchor| anchor.index).max().unwrap_or(0) + 1;
    let mut slots = vec![None; slot_count];
    if chart_h <= 0.0 || !chart_h.is_finite() {
        return slots;
    }

    anchors.sort_by(|a, b| a.y.total_cmp(&b.y).then_with(|| a.index.cmp(&b.index)));

    let min_y = SERIES_LABEL_HEIGHT * 0.5 + SERIES_LABEL_MARGIN;
    let max_y = (chart_h - SERIES_LABEL_HEIGHT * 0.5 - SERIES_LABEL_MARGIN).max(min_y);
    let step = SERIES_LABEL_HEIGHT + SERIES_LABEL_GAP;
    let mut positions = Vec::with_capacity(anchors.len());
    let mut next_y = min_y;

    for anchor in anchors {
        let desired_y = anchor.y.clamp(min_y, max_y);
        let label_y = desired_y.max(next_y);
        positions.push((anchor.index, label_y));
        next_y = label_y + step;
    }

    if positions
        .last()
        .is_some_and(|(_, label_y)| *label_y > max_y)
    {
        let mut next_y = max_y;
        for (_, label_y) in positions.iter_mut().rev() {
            *label_y = (*label_y).min(next_y);
            next_y = *label_y - step;
        }

        if let Some((_, first_y)) = positions.first()
            && *first_y < min_y
        {
            let shift = min_y - *first_y;
            for (_, label_y) in &mut positions {
                *label_y += shift;
            }
        }
    }

    for (index, label_y) in positions {
        if let Some(slot) = slots.get_mut(index) {
            *slot = Some(label_y);
        }
    }
    slots
}
