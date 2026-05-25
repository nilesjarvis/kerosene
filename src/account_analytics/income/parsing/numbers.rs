use crate::helpers::parse_finite_number;

pub(in crate::account_analytics::income) fn parse_f64_str(value: &str) -> Option<f64> {
    parse_finite_number(value)
}

#[cfg(test)]
mod tests;
