use super::*;

#[test]
fn selected_hip3_fetch_scope_reduces_estimated_request_weight() {
    let all_markets = AccountDataFetchScope::all_markets(["xyz", "flx", "new"]);
    let selected = AccountDataFetchScope::hip3_dex("XYZ");

    assert_eq!(selected.selected_hip3_dex(), Some("xyz"));
    assert!(selected.estimated_info_weight() < all_markets.estimated_info_weight());
    assert!(!selected.fetches_main_open_orders());
    assert_eq!(all_markets.hip3_dexes(&[]), ["flx", "new", "xyz"]);
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

#[test]
fn dex_list_preserves_stored_order_duplicates_and_literal_values() {
    let scope = AccountDataFetchScope::AllMarkets {
        hip3_dexes: ["XYZ", "", "xyz", "XYZ", " ÄBC "]
            .into_iter()
            .map(str::to_string)
            .collect(),
    };
    for fallback in [&[][..], &["fallback", "other"][..]] {
        assert_eq!(
            scope.hip3_dexes(fallback),
            ["XYZ", "", "xyz", "XYZ", " ÄBC "]
        );
    }
    assert_eq!(
        AccountDataFetchScope::all_markets([" XYZ ", "flx", "xyz", " "]).hip3_dexes(&["fallback"]),
        ["flx", "xyz"]
    );
}

#[test]
fn empty_all_markets_dex_list_uses_exact_caller_fallback() {
    let scope = AccountDataFetchScope::AllMarkets {
        hip3_dexes: Vec::new(),
    };
    let fallback = ["Zeta".to_string(), "".to_string(), "Zeta".to_string()];
    let fallback_refs = fallback.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(scope.hip3_dexes(&fallback_refs), ["Zeta", "", "Zeta"]);
    assert!(scope.hip3_dexes(&[]).is_empty());
    assert!(scope.fetches_main_open_orders());
}

#[test]
fn selected_dex_list_never_uses_fallback_or_renormalizes_stored_value() {
    for dex in ["", " XYZ ", "ÄBC"] {
        let scope = AccountDataFetchScope::Hip3Dex {
            dex: dex.to_string(),
        };
        assert_eq!(scope.hip3_dexes(&["fallback"]), [dex]);
        assert_eq!(scope.hip3_dexes(&[]), [dex]);
        assert!(!scope.fetches_main_open_orders());
    }
}
