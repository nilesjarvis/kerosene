use super::{BuilderCodeConfig, BuilderFee};

const BUILDER: &str = "0xabcdef0123456789abcdef0123456789abcdef01";

#[test]
fn builder_defaults_are_disabled_and_legacy_fields_default() {
    let empty: BuilderCodeConfig = serde_json::from_str("{}").expect("legacy empty config");
    assert_eq!(empty, BuilderCodeConfig::default());
    assert_eq!(empty.resolve().expect("disabled builder"), None);
    assert_eq!(
        serde_json::to_value(empty).expect("config serializes"),
        serde_json::json!({"enabled": false, "address": "", "fee_tenths_bps": 0})
    );
}

#[test]
fn enabled_invalid_builder_never_silently_disables() {
    for config in [
        BuilderCodeConfig {
            enabled: true,
            ..BuilderCodeConfig::default()
        },
        BuilderCodeConfig {
            enabled: true,
            address: BUILDER.to_string(),
            fee_tenths_bps: 0,
        },
        BuilderCodeConfig {
            enabled: true,
            address: BUILDER.to_string(),
            fee_tenths_bps: 101,
        },
    ] {
        assert!(config.resolve().is_err());
    }
}

#[test]
fn valid_config_roundtrips_and_normalizes_only_on_resolution() {
    let config = BuilderCodeConfig {
        enabled: true,
        address: format!(" 0x{} ", BUILDER[2..].to_ascii_uppercase()),
        fee_tenths_bps: 37,
    };
    let json = serde_json::to_string(&config).expect("serializes");
    let roundtrip: BuilderCodeConfig = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(roundtrip, config);
    let fee = roundtrip.resolve().expect("valid config").expect("enabled");
    assert_eq!(fee.address(), BUILDER);
    assert_eq!(fee.fee_tenths_bps(), 37);
    assert_eq!(fee.fee_bps_text(), "3.7");
    assert_eq!(fee.max_fee_rate(), "0.037%");
}

#[test]
fn builder_fee_parser_uses_exact_decimal_units() {
    for (input, expected, bps_text, percent) in [
        ("0.1", 1, "0.1", "0.001%"),
        ("0.9", 9, "0.9", "0.009%"),
        (" 1 ", 10, "1", "0.010%"),
        ("1.1", 11, "1.1", "0.011%"),
        ("3.3", 33, "3.3", "0.033%"),
        ("09.9", 99, "9.9", "0.099%"),
        ("10.0", 100, "10", "0.100%"),
    ] {
        let fee = BuilderFee::from_inputs(BUILDER, input).expect("valid exact decimal fee");
        assert_eq!(fee.fee_tenths_bps(), expected, "{input}");
        assert_eq!(fee.fee_bps_text(), bps_text);
        assert_eq!(fee.max_fee_rate(), percent);
    }
}

#[test]
fn builder_fee_parser_rejects_ambiguous_precision_and_overflow() {
    for input in [
        "",
        " ",
        "0",
        "0.0",
        "10.1",
        "11",
        "-1",
        "+1",
        "-0.1",
        "1e0",
        "NaN",
        "inf",
        ".1",
        "1.",
        "1.00",
        "0.01",
        "1.2.3",
        "1,2",
        "1 2",
        "１",
        "1.１",
        "4294967296",
        "429496730",
        "9999999999999999999999999999999999999999999999999999999",
    ] {
        assert!(
            BuilderFee::from_inputs(BUILDER, input).is_err(),
            "accepted {input:?}"
        );
    }
}

#[test]
fn builder_address_requires_nonzero_exact_twenty_bytes_without_echoing_input() {
    for address in [
        "",
        "not-an-address",
        "0x",
        "0x0000000000000000000000000000000000000000",
        "0xabcdef0123456789abcdef0123456789abcdef0123",
        "abcdef0123456789abcdef0123456789abcdef01",
        "0xabcdef0123456789abcdef0123456789abcdef0z",
    ] {
        let error = BuilderFee::new(address, 10).expect_err("invalid address");
        if address.len() > 3 {
            assert!(!error.contains(address));
        }
    }
}

#[test]
fn builder_debug_redacts_addresses() {
    let config = BuilderCodeConfig {
        enabled: true,
        address: BUILDER.to_string(),
        fee_tenths_bps: 10,
    };
    let fee = config
        .resolve()
        .expect("valid builder")
        .expect("enabled builder");
    for debug in [format!("{config:?}"), format!("{fee:?}")] {
        assert!(!debug.contains(BUILDER));
        assert!(debug.contains("<redacted>"));
    }
}
