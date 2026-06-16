use serde_json::Value;

pub(crate) type L2BookSigfigs = (Option<u8>, Option<u8>);

pub(crate) fn l2_book_sigfigs_from_value(value: &Value) -> L2BookSigfigs {
    (
        l2_book_u8_field(value, "nSigFigs"),
        l2_book_u8_field(value, "mantissa"),
    )
}

pub(crate) fn l2_book_payload_matches_sigfigs(value: &Value, expected: L2BookSigfigs) -> bool {
    let actual = l2_book_sigfigs_from_value(value);
    actual == (None, None) || actual == expected
}

fn l2_book_u8_field(value: &Value, field: &str) -> Option<u8> {
    value
        .get(field)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str()?.parse::<u64>().ok())
        })
        .and_then(|value| u8::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_book_sigfigs_parse_numeric_and_string_fields() {
        let value = serde_json::json!({
            "coin": "BTC",
            "nSigFigs": 5,
            "mantissa": "2"
        });

        assert_eq!(l2_book_sigfigs_from_value(&value), (Some(5), Some(2)));
    }

    #[test]
    fn l2_book_payload_match_treats_missing_precision_as_unattributed() {
        let value = serde_json::json!({ "coin": "BTC" });

        assert!(l2_book_payload_matches_sigfigs(&value, (Some(5), Some(2))));
    }

    #[test]
    fn l2_book_payload_match_rejects_conflicting_echoed_precision() {
        let value = serde_json::json!({
            "coin": "BTC",
            "nSigFigs": 5,
            "mantissa": 2
        });

        assert!(l2_book_payload_matches_sigfigs(&value, (Some(5), Some(2))));
        assert!(!l2_book_payload_matches_sigfigs(&value, (Some(4), Some(2))));
        assert!(!l2_book_payload_matches_sigfigs(&value, (Some(5), None)));
    }

    #[test]
    fn l2_book_payload_match_requires_partial_precision_to_match_exactly() {
        let value = serde_json::json!({
            "coin": "BTC",
            "nSigFigs": 5
        });

        assert!(l2_book_payload_matches_sigfigs(&value, (Some(5), None)));
        assert!(!l2_book_payload_matches_sigfigs(&value, (Some(5), Some(2))));
        assert!(!l2_book_payload_matches_sigfigs(&value, (Some(4), None)));
    }
}
