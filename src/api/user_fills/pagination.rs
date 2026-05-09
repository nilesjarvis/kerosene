use super::model::UserFillsRequest;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// User Fill Pagination
// ---------------------------------------------------------------------------

pub(super) fn next_user_fills_request(
    start_time: u64,
    fetched_count: usize,
    oldest_time: u64,
    newest_time: u64,
) -> Option<UserFillsRequest> {
    if fetched_count < 2000 || oldest_time <= start_time {
        return None;
    }

    let next_end_time = if oldest_time == newest_time {
        oldest_time.saturating_sub(1)
    } else {
        oldest_time
    };

    if next_end_time < start_time {
        None
    } else {
        Some(UserFillsRequest {
            start_time,
            end_time: Some(next_end_time),
        })
    }
}
