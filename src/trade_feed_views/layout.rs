pub(super) const ROW_HEIGHT: f32 = 24.0;
pub(super) const CELL_INSET: f32 = 8.0;
pub(super) const SIDE_WIDTH: f32 = 32.0;
pub(super) const TRADER_WIDTH: f32 = 112.0;
pub(super) const TIME_WIDTH: f32 = 112.0;
pub(super) const CONTEXT_WIDTH: f32 = 44.0;
pub(super) const GROUP_COUNT_WIDTH: f32 = 32.0;

#[derive(Clone, Copy, Debug)]
pub(super) struct TapeRowLayout {
    pub(super) show_time: bool,
    pub(super) show_price: bool,
    pub(super) show_value: bool,
    pub(super) show_context: bool,
    pub(super) gap: f32,
}

impl TapeRowLayout {
    pub(super) fn new(width: f32, show_context: bool, notional: bool) -> Self {
        let gap = if width >= 480.0 { 8.0 } else { 4.0 };
        // Keep side, trader (including group count), action and size first.
        // Optional columns are admitted only when each numeric column still fits.
        let mut required = CELL_INSET * 2.0 + SIDE_WIDTH + TRADER_WIDTH + 64.0 + gap * 2.0;
        if show_context {
            required += CONTEXT_WIDTH + gap;
        }
        let show_price = width >= required + 96.0 + gap;
        if show_price {
            required += 96.0 + gap;
        }
        let show_time = show_price && width >= required + TIME_WIDTH + gap;
        if show_time {
            required += TIME_WIDTH + gap;
        }
        let show_value = !notional && show_time && width >= required + 96.0 + gap;
        Self {
            show_time,
            show_price,
            show_value,
            show_context,
            gap,
        }
    }

    pub(super) fn row_count(height: f32) -> usize {
        ((height.max(0.0) / ROW_HEIGHT).floor() as usize).min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_columns_leave_room_for_core_values_and_context() {
        for context in [false, true] {
            for notional in [false, true] {
                for width in [280.0, 320.0, 400.0, 480.0, 640.0, 1000.0] {
                    let layout = TapeRowLayout::new(width, context, notional);
                    let mut columns = 3;
                    let mut occupied = CELL_INSET * 2.0 + SIDE_WIDTH + TRADER_WIDTH + 64.0;
                    for (shown, cell_width) in [
                        (context, CONTEXT_WIDTH),
                        (layout.show_price, 96.0),
                        (layout.show_time, TIME_WIDTH),
                        (layout.show_value, 96.0),
                    ] {
                        if shown {
                            occupied += cell_width;
                            columns += 1;
                        }
                    }
                    occupied += layout.gap * (columns - 1) as f32;
                    assert!(occupied <= width, "{width}: {layout:?}");
                    assert!(!notional || !layout.show_value);
                }
            }
        }
        assert!(!TapeRowLayout::new(280.0, true, false).show_price);
        assert!(TapeRowLayout::new(640.0, true, false).show_value);
    }

    #[test]
    fn only_complete_rows_are_rendered_in_the_measured_body() {
        for (height, count) in [
            (-1.0, 0),
            (0.0, 0),
            (23.9, 0),
            (24.0, 1),
            (47.9, 1),
            (240.0, 10),
            (10_000.0, 100),
        ] {
            assert_eq!(TapeRowLayout::row_count(height), count);
        }
    }
}
