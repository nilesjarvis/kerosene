use super::model::UserFillsRequest;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// User Fill Pagination
// ---------------------------------------------------------------------------

pub(super) fn next_user_fills_request(
    start_time: u64,
    end_time: u64,
    fetched_count: usize,
    newest_time: u64,
) -> (Option<UserFillsRequest>, Option<String>) {
    if fetched_count < 2000 || newest_time >= end_time {
        return (None, None);
    }

    let (next_start_time, progress_warning) = if newest_time <= start_time {
        let Some(next_start_time) = newest_time.checked_add(1) else {
            return (
                None,
                Some(
                    "Journal history pagination stopped because the API returned a full page \
                     without timestamp progress."
                        .to_string(),
                ),
            );
        };
        (
            next_start_time,
            Some(format!(
                "Journal history pagination advanced past timestamp {newest_time} because the API \
                 returned a full page without timestamp progress. Fills sharing that exact \
                 millisecond may be incomplete."
            )),
        )
    } else {
        (newest_time, None)
    };

    if next_start_time > end_time {
        (None, progress_warning)
    } else {
        (
            Some(UserFillsRequest {
                start_time: next_start_time,
                end_time: Some(end_time),
            }),
            progress_warning,
        )
    }
}
