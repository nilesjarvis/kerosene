use super::*;
use crate::api::OutcomeContract;

fn registry() -> Vec<OutcomeTemplate> {
    serde_json::from_str(include_str!("fixtures/templates.json")).expect("public template registry")
}

fn metadata() -> OutcomeMetaResponse {
    serde_json::from_str(include_str!("fixtures/hip4.json")).expect("public HIP-4 metadata")
}

fn live_symbols() -> Vec<ExchangeSymbol> {
    parse_outcome_symbols(metadata(), &registry()).expect("valid public market snapshot")
}

#[test]
fn public_hip4_snapshot_resolves_all_tradable_contracts_and_exact_identities() {
    let symbols = live_symbols();
    assert_eq!(symbols.len(), metadata().outcomes.len() * 2);
    for symbol in &symbols {
        let info = symbol.outcome.as_ref().expect("outcome terms");
        assert_eq!(
            symbol.key,
            format!("#{}", info.outcome_id * 10 + info.side_index)
        );
        assert_eq!(symbol.asset_index, 100_000_000 + info.encoding);
        if info.is_question_fallback {
            assert!(info.trading_block_reason(0).is_some());
            continue;
        }
        assert!(info.contract.verified);
        assert!(
            info.contract.blocked_reason.is_none(),
            "{}: {:?}",
            symbol.key,
            info.contract.blocked_reason
        );
        assert!(
            info.contract.deadline_ms.is_some(),
            "{} has no deadline",
            symbol.key
        );
        assert!(
            !info.display_label().contains("template:"),
            "{}",
            info.display_label()
        );
        assert!(!info.side_name.contains('{'));
        if info.outcome_name.starts_with("template:") {
            let rules = info.contract.rules.as_deref().expect("resolved rules");
            assert!(!rules.contains('{'));
            assert!(!rules.contains(" metadata="));
        }
    }
}

#[test]
fn touch_ipo_sports_and_policy_questions_render_their_actual_terms() {
    let symbols = live_symbols();
    let touch = outcome_by_key_or_panic(&symbols, "#12090");
    assert!(touch.market_label().contains("HYPE touches 100"));
    assert!(
        touch
            .contract
            .rules
            .as_deref()
            .expect("rules")
            .contains("settle as soon as a touch occurs")
    );
    let ipo = outcome_by_key_or_panic(&symbols, "#25970");
    assert!(ipo.market_label().contains("Anthropic IPO confirmed"));
    let arsenal = outcome_by_key_or_panic(&symbols, "#14730");
    assert_eq!(arsenal.side_condition_short_label(), "Arsenal");
    assert!(arsenal.market_label().contains("Premier League"));
    assert!(
        arsenal
            .contract
            .rules
            .as_deref()
            .expect("parent and child rules")
            .contains("Tournament Result")
    );
    let policy = outcome_by_key_or_panic(&symbols, "#36000");
    assert_eq!(policy.side_condition_short_label(), "No change");
    assert!(policy.market_label().contains("rate decision"));
    assert_eq!(
        policy.contract.deadline_ms,
        templates::parse_deadline("20261209-2300")
    );
    let lions = outcome_by_key_or_panic(&symbols, "#29920");
    let bills = outcome_by_key_or_panic(&symbols, "#29921");
    assert_eq!(lions.side_name, "Lions");
    assert_eq!(bills.side_name, "Bills");
    assert!(!bills.side_condition_label().starts_with("not "));
    assert!(bills.side_condition_label().contains("Bills"));
}

#[test]
fn deadline_boundary_and_settlement_gate_do_not_confuse_sports_start_with_expiry() {
    let symbols = live_symbols();
    let sports = outcome_by_key_or_panic(&symbols, "#29920");
    let start = templates::parse_deadline("20260918-0015").expect("start");
    let deadline = templates::parse_deadline("20260918-0615").expect("resolution deadline");
    assert_eq!(sports.contract.deadline_ms, Some(deadline));
    assert_eq!(sports.trading_block_reason(start + 1), None);
    assert_eq!(sports.trading_block_reason(deadline - 1), None);
    assert_eq!(
        sports.trading_block_reason(deadline),
        Some("Resolution deadline passed; awaiting settlement")
    );
    let mut binary = outcome_by_key_or_panic(&symbols, "#28960").clone();
    let expiry = binary.contract.deadline_ms.expect("expiry");
    assert_eq!(binary.trading_block_reason(expiry - 1), None);
    assert_eq!(
        binary.trading_block_reason(expiry),
        Some("Expired; awaiting settlement")
    );
    binary
        .question_settled_named_outcomes
        .push(binary.outcome_id);
    assert_eq!(binary.trading_block_reason(0), Some("Settled"));
}

#[test]
fn scalar_contracts_keep_custom_sides_and_fractional_payout_rules() {
    let meta = outcome_meta_from_json(serde_json::json!({
        "outcomes": [{"outcome": 42, "name": "template:scalarPrice",
            "description": "perp:BTC|low:60000|high:100000|time:20300101-0000|seconds:90|priceDescription:the Hyperliquid BTC perp",
            "sideSpecs": [{"name": "template:Long"}, {"name": "template:Short"}], "quoteToken": "USDC"}]
    }));
    let symbols = parse_outcome_symbols(meta, &registry()).expect("scalar market");
    let short = outcome_by_key_or_panic(&symbols, "#421");
    assert!(short.contract.scalar);
    assert_eq!(short.side_name, "Short");
    assert!(!short.side_condition_label().starts_with("not "));
    assert!(
        short
            .contract
            .rules
            .as_deref()
            .expect("rules")
            .contains("Short tokens pay out $1 minus the Long payout")
    );
    assert_eq!(short.trading_block_reason(0), None);
}

#[test]
fn unsafe_contracts_remain_readable_but_are_not_orderable() {
    let base = metadata()
        .outcomes
        .into_iter()
        .find(|entry| entry.outcome == 2896)
        .expect("Skew binary");
    for case in [
        "unknown template",
        "missing parameter",
        "side mismatch",
        "missing registry",
        "unknown quote",
    ] {
        let mut entry = base.clone();
        let mut templates = registry();
        match case {
            "unknown template" => entry.name = "template:futureContract".into(),
            "missing parameter" => entry.description = "perp:BTC".into(),
            "side mismatch" => entry.side_specs[1].name = "template:Maybe".into(),
            "missing registry" => templates.clear(),
            "unknown quote" => entry.quote_token = "OTHER".into(),
            _ => unreachable!(),
        }
        let symbols = parse_outcome_symbols(
            OutcomeMetaResponse {
                outcomes: vec![entry],
                questions: vec![],
                fee_scale: None,
            },
            &templates,
        )
        .expect("identity is still inspectable");
        let info = outcome_by_key_or_panic(&symbols, "#28960");
        assert!(info.trading_block_reason(0).is_some(), "{case}");
        assert!(!info.display_label().contains("template:"));
    }
}

#[test]
fn child_contract_requires_the_matching_valid_parent_question() {
    for case in ["missing", "wrong", "invalid"] {
        let mut meta = metadata();
        meta.outcomes.retain(|entry| entry.outcome == 1473);
        match case {
            "missing" => meta.questions.clear(),
            "wrong" => meta
                .questions
                .iter_mut()
                .filter(|q| q.named_outcomes.contains(&1473))
                .for_each(|q| q.name = "template:policyRateDecision".into()),
            "invalid" => meta
                .questions
                .iter_mut()
                .filter(|q| q.named_outcomes.contains(&1473))
                .for_each(|q| q.description = "season:2026".into()),
            _ => unreachable!(),
        }
        let symbols = parse_outcome_symbols(meta, &registry()).expect("inspectable metadata");
        assert!(
            outcome_by_key_or_panic(&symbols, "#14730")
                .trading_block_reason(0)
                .is_some(),
            "{case}"
        );
    }
}

#[test]
fn malformed_identity_and_question_membership_fail_the_metadata_family() {
    for case in [
        "duplicate outcome",
        "overflow",
        "one side",
        "duplicate question",
        "multiple parents",
        "fallback overlap",
    ] {
        let mut meta = metadata();
        match case {
            "duplicate outcome" => meta.outcomes.push(meta.outcomes[0].clone()),
            "overflow" => meta.outcomes[0].outcome = u32::MAX,
            "one side" => {
                meta.outcomes[0].side_specs.pop();
            }
            "duplicate question" => meta.questions.push(meta.questions[0].clone()),
            "multiple parents" => {
                let member = meta.questions[0].named_outcomes[0];
                meta.questions[1].named_outcomes.push(member);
            }
            "fallback overlap" => {
                meta.questions[0].fallback_outcome = Some(meta.questions[0].named_outcomes[0])
            }
            _ => unreachable!(),
        }
        assert!(parse_outcome_symbols(meta, &registry()).is_err(), "{case}");
    }
    assert!(
        serde_json::from_value::<OutcomeMetaResponse>(serde_json::json!({"questions":[]})).is_err()
    );
}

#[test]
fn legacy_bucket_rules_reject_missing_thresholds_and_duplicate_parameters() {
    for description in [
        "class:priceBucket|underlying:BTC|expiry:20300101-0000|priceThresholds:70000,,80000",
        "class:priceBucket|underlying:BTC|expiry:20300101-0000|priceThresholds:80000,70000",
        "class:priceBucket|underlying:BTC|expiry:20300101-0000|priceThresholds:70000,80000|underlying:ETH",
    ] {
        let meta = outcome_meta_from_json(serde_json::json!({
            "outcomes": [{"outcome": 42, "name": "Recurring Named Outcome", "description": "index:1", "sideSpecs": [{"name":"Yes"},{"name":"No"}], "quoteToken":"USDC"}],
            "questions": [{"question":1, "name":"Recurring", "description":description, "namedOutcomes":[42]}]
        }));
        let symbols =
            parse_outcome_symbols(meta, &registry()).expect("identity remains inspectable");
        assert!(
            outcome_by_key_or_panic(&symbols, "#420")
                .trading_block_reason(0)
                .is_some()
        );
    }
}

#[test]
fn fees_use_valid_published_scales_and_never_invent_missing_values_or_rebates() {
    let symbols = live_symbols();
    let mut info = outcome_by_key_or_panic(&symbols, "#28960").clone();
    assert_eq!(info.contract.fee_scale.as_deref(), Some("1.0"));
    assert_eq!(info.contract.deployer_fee_scale.as_deref(), Some("1.0"));
    assert!(info.fee_terms_label().contains("no maker rebates"));
    for invalid in [None, Some("NaN"), Some("inf"), Some("-1"), Some("")] {
        assert_eq!(valid_fee_scale(invalid), None);
    }
    assert_eq!(valid_fee_scale(Some("0")), Some("0".into()));
    info.contract = OutcomeContract::verified_fixture();
    assert!(info.fee_terms_label().contains("deployer: unavailable"));
    info.contract.verified = false;
    assert_eq!(info.fee_terms_label(), "Live outcome fee terms unavailable");
}
