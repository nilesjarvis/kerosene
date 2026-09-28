use super::*;

#[test]
fn selected_hip3_fetch_scope_reduces_estimated_request_weight() {
    let all_markets = AccountDataFetchScope::all_markets(["xyz", "flx", "new"]);
    let selected = AccountDataFetchScope::hip3_dex("XYZ");

    assert_eq!(selected.selected_hip3_dex(), Some("xyz"));
    assert!(selected.estimated_info_weight() < all_markets.estimated_info_weight());
    assert!(!selected.fetches_main_open_orders());
    assert_eq!(
        all_markets.hip3_dexes(&[]),
        vec!["flx".to_string(), "new".to_string(), "xyz".to_string()]
    );
}

#[test]
fn automatic_refresh_interval_increases_with_heavier_scope() {
    let all_markets = AccountDataFetchScope::all_markets(["xyz", "flx", "new"]);
    let selected = AccountDataFetchScope::hip3_dex("XYZ");

    assert!(
        all_markets.automatic_refresh_interval_secs() > selected.automatic_refresh_interval_secs()
    );
}

#[test]
fn selected_scope_normalizes_borrowed_and_owned_inputs_identically() {
    for (input, normalized) in [
        ("", None),
        (" \t\n ", None),
        ("xyz", Some("xyz")),
        (" XYZ ", Some("xyz")),
        ("MiXeD", Some("mixed")),
        (" ÄBC ", Some("Äbc")),
    ] {
        let expected = normalized
            .map(|dex| AccountDataFetchScope::Hip3Dex {
                dex: dex.to_string(),
            })
            .unwrap_or_default();
        let owned = input.to_string();
        for scope in [
            AccountDataFetchScope::hip3_dex(input),
            AccountDataFetchScope::hip3_dex(&owned),
            AccountDataFetchScope::hip3_dex(owned),
        ] {
            assert_eq!(scope, expected);
        }
    }
}
