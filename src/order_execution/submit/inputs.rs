// ---------------------------------------------------------------------------
// Order Input Helpers
// ---------------------------------------------------------------------------

use crate::helpers::parse_positive_number;

#[cfg(test)]
mod tests;

pub(super) fn parse_positive_amount(input: &str) -> Option<f64> {
    parse_positive_number(input)
}
