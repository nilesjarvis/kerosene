// ---------------------------------------------------------------------------
// Order Status Results
// ---------------------------------------------------------------------------

mod oid;
mod placement;

fn returned_cloid_mismatches(status: &crate::api::OrderStatusResult, expected_cloid: &str) -> bool {
    status
        .cloid
        .as_deref()
        .is_some_and(|cloid| !cloid.eq_ignore_ascii_case(expected_cloid))
}

fn returned_oid_mismatches(status: &crate::api::OrderStatusResult, expected_oid: u64) -> bool {
    status.oid.is_some_and(|oid| oid != expected_oid)
}
