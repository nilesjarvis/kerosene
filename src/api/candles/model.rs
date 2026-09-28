use serde::{Deserialize, Serialize, de};
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct Candle {
    #[serde(rename = "t")]
    pub open_time: u64,
    #[serde(rename = "T")]
    pub close_time: u64,
    #[serde(rename = "o", deserialize_with = "de_string_or_number_to_f64")]
    pub open: f64,
    #[serde(rename = "h", deserialize_with = "de_string_or_number_to_f64")]
    pub high: f64,
    #[serde(rename = "l", deserialize_with = "de_string_or_number_to_f64")]
    pub low: f64,
    #[serde(rename = "c", deserialize_with = "de_string_or_number_to_f64")]
    pub close: f64,
    #[serde(rename = "v", deserialize_with = "de_string_or_number_to_f64")]
    pub volume: f64,
}

impl fmt::Debug for Candle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Candle")
            .field("open_time", &self.open_time)
            .field("close_time", &self.close_time)
            .field("open", &"<redacted>")
            .field("high", &"<redacted>")
            .field("low", &"<redacted>")
            .field("close", &"<redacted>")
            .field("volume", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
impl Candle {
    pub(crate) fn test_price(open_time: u64, close: f64) -> Self {
        Self::test_ohlcv(
            open_time,
            open_time + 59_999,
            [close, close + 1.0, close - 1.0, close],
            10.0,
        )
    }

    pub(crate) fn test_flat(open_time: u64, value: f64) -> Self {
        Self::test_ohlcv(
            open_time,
            open_time + 59_999,
            [value, value, value, value],
            1.0,
        )
    }

    pub(crate) fn test_ohlcv(
        open_time: u64,
        close_time: u64,
        [open, high, low, close]: [f64; 4],
        volume: f64,
    ) -> Self {
        Self {
            open_time,
            close_time,
            open,
            high,
            low,
            close,
            volume,
        }
    }
}

/// Deserialize a value that may be either a JSON string (e.g. `"29258.0"`)
/// or a JSON number (e.g. `29258.0`) into an f64. The Hyperliquid API is
/// inconsistent: some assets return strings, others return raw numbers.
pub fn de_string_or_number_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringOrNumber;

    impl<'de> de::Visitor<'de> for StringOrNumber {
        type Value = f64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or number representing an f64")
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
            v.parse::<f64>().map_err(de::Error::custom)
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<f64, E> {
            v.parse::<f64>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

#[cfg(test)]
mod tests {
    use super::Candle;
    use serde::Deserialize;
    use serde_json::json;

    #[test]
    fn owned_and_borrowed_candle_values_preserve_numeric_forms_and_errors() {
        let original = json!({
            "t": 10, "T": 20, "o": 1, "h": 2, "l": 0.5, "c": 1.5, "v": 12,
            "unknown": { "nested": [null, true] }
        });
        for (index, field) in ["o", "h", "l", "c", "v"].into_iter().enumerate() {
            for (value, expected) in [
                (json!(42), 42.0),
                (json!(-42), -42.0),
                (json!(u64::MAX), u64::MAX as f64),
                (json!(1.25), 1.25),
                (json!("1.25e2"), 125.0),
                (json!("-0"), -0.0),
                (json!("NaN"), f64::NAN),
                (json!("inf"), f64::INFINITY),
            ] {
                let mut data = original.clone();
                data[field] = value;
                let mut expected_fields = [1.0_f64, 2.0, 0.5, 1.5, 12.0];
                expected_fields[index] = expected;
                for candle in [
                    serde_json::from_value::<Candle>(data.clone()).expect("owned candle"),
                    Candle::deserialize(&data).expect("borrowed candle"),
                ] {
                    assert_eq!((candle.open_time, candle.close_time), (10, 20));
                    assert_eq!(
                        [
                            candle.open,
                            candle.high,
                            candle.low,
                            candle.close,
                            candle.volume
                        ]
                        .map(f64::to_bits),
                        expected_fields.map(f64::to_bits)
                    );
                }
            }
            for malformed in [json!(null), json!(true), json!({}), json!([]), json!("bad")] {
                let mut data = original.clone();
                data[field] = malformed;
                let owned = serde_json::from_value::<Candle>(data.clone())
                    .expect_err("invalid owned numeric field");
                let borrowed =
                    Candle::deserialize(&data).expect_err("invalid borrowed numeric field");
                assert_eq!(owned.to_string(), borrowed.to_string());
            }
        }
    }

    #[test]
    fn candle_debug_redacts_ohlcv_payload() {
        let candle = Candle::test_ohlcv(
            1_700_000_000_000,
            1_700_000_059_999,
            [12345.67, 12355.89, 12340.12, 12350.34],
            98765.43,
        );

        let rendered = format!("{candle:?}");

        assert!(rendered.contains("open_time: 1700000000000"));
        assert!(rendered.contains("close_time: 1700000059999"));
        assert!(rendered.contains("open: \"<redacted>\""));
        assert!(rendered.contains("volume: \"<redacted>\""));
        for secret in ["12345.67", "12355.89", "12340.12", "12350.34", "98765.43"] {
            assert!(!rendered.contains(secret), "candle Debug leaked {secret}");
        }
    }
}
