use super::OrderKind;

#[test]
fn order_kind_config_strings_round_trip_all_variants() {
    for kind in [
        OrderKind::Market,
        OrderKind::Limit,
        OrderKind::Chase,
        OrderKind::Twap,
        OrderKind::LimitIoc,
    ] {
        assert_eq!(OrderKind::from_config_str(kind.config_str()), kind);
    }
}

#[test]
fn order_kind_config_parser_preserves_limit_ioc_aliases() {
    assert_eq!(OrderKind::from_config_str("Limit IOC"), OrderKind::LimitIoc);
    assert_eq!(OrderKind::from_config_str("LimitIoc"), OrderKind::LimitIoc);
    assert_eq!(OrderKind::from_config_str("IOC"), OrderKind::LimitIoc);
}

#[test]
fn order_kind_config_parser_defaults_unknown_values_to_limit() {
    assert_eq!(OrderKind::from_config_str(""), OrderKind::Limit);
    assert_eq!(OrderKind::from_config_str("Unknown"), OrderKind::Limit);
}
