// ---------------------------------------------------------------------------
// Position Transition Helpers
// ---------------------------------------------------------------------------

pub(super) const POSITION_EPSILON: f64 = 1e-6;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FillPositionTransition {
    pub(super) new_pos: f64,
    pub(super) is_flip: bool,
    pub(super) is_close: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ResolvedStartPosition {
    pub(super) start_pos: f64,
    pub(super) same_timestamp_mismatch: bool,
}

pub(super) fn is_non_perp_coin(coin: &str) -> bool {
    coin.starts_with('@') || coin.starts_with('#')
}

pub(super) fn signed_fill_size(side: &str, size: f64) -> f64 {
    if side == "A" { -size } else { size }
}

pub(super) fn resolved_start_position(
    api_start_pos: f64,
    tracked_position: Option<(u64, f64)>,
    fill_time: u64,
) -> ResolvedStartPosition {
    match tracked_position {
        Some((last_time, position))
            if last_time == fill_time && (position - api_start_pos).abs() <= POSITION_EPSILON =>
        {
            ResolvedStartPosition {
                start_pos: position,
                same_timestamp_mismatch: false,
            }
        }
        Some((last_time, _position)) if last_time == fill_time => ResolvedStartPosition {
            start_pos: api_start_pos,
            same_timestamp_mismatch: true,
        },
        _ => ResolvedStartPosition {
            start_pos: api_start_pos,
            same_timestamp_mismatch: false,
        },
    }
}

pub(super) fn fill_position_transition(
    start_pos: f64,
    signed_size: f64,
    is_settlement: bool,
) -> FillPositionTransition {
    let new_pos = if is_settlement {
        start_pos
    } else {
        start_pos + signed_size
    };
    let is_flip = (start_pos > POSITION_EPSILON && new_pos < -POSITION_EPSILON)
        || (start_pos < -POSITION_EPSILON && new_pos > POSITION_EPSILON);
    let is_close = new_pos.abs() < POSITION_EPSILON;

    FillPositionTransition {
        new_pos,
        is_flip,
        is_close,
    }
}
