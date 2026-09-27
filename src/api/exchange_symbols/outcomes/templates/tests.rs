use super::*;

fn registry() -> Vec<OutcomeTemplate> {
    serde_json::from_str(include_str!("../tests/fixtures/templates.json"))
        .expect("template registry")
}

const BINARY: &str =
    "perp:BTC|threshold:75000|time:20300101-0000|seconds:90|priceDescription:the BTC perp";

#[test]
fn template_parameters_reject_ambiguous_and_invalid_terms() {
    for description in [
        BINARY.replace("threshold:75000", "threshold:NaN"),
        BINARY.replace("threshold:75000", "threshold:-1"),
        BINARY.replace("threshold:75000", "threshold:1e309"),
        BINARY.replace("time:20300101-0000", "time:20300230-0000"),
        BINARY.replace("time:20300101-0000", "time:19690101-0000"),
        BINARY.replace("seconds:90", "seconds:0"),
        BINARY.replace("seconds:90", "seconds:1.5"),
        BINARY.replace("perp:BTC", "perp:"),
        BINARY.replace("perp:BTC|", ""),
        format!("{BINARY}|perp:ETH"),
        format!("{BINARY}|unstructured text"),
    ] {
        assert!(
            resolve_template("template:binaryPrice", &description, &registry()).is_err(),
            "{description}"
        );
    }
}

#[test]
fn unknown_duplicate_and_unrenderable_templates_fail_closed() {
    let mut templates = registry();
    assert!(resolve_template("template:newType", BINARY, &templates).is_err());
    let binary = templates
        .iter()
        .find(|t| t.id == "binaryPrice")
        .expect("binary")
        .clone();
    templates.push(binary.clone());
    assert!(resolve_template("template:binaryPrice", BINARY, &templates).is_err());
    for (name, description, kind) in [
        ("{missing}", "rule", "hlPerp"),
        ("{perp", "rule", "hlPerp"),
        ("", "rule", "hlPerp"),
        ("{perp}", "", "hlPerp"),
        ("{perp}", "rule", "futureType"),
    ] {
        let mut template = binary.clone();
        template.name = name.into();
        template.description = description.into();
        template.keywords[0].1 = kind.into();
        assert!(resolve_template("template:binaryPrice", BINARY, &[template]).is_err());
    }
}

#[test]
fn interpolation_does_not_recursively_expand_parameter_text() {
    let values = HashMap::from([("a", "{b}".into()), ("b", "replacement".into())]);
    assert_eq!(
        interpolate("value: {a}", &values).expect("single pass"),
        "value: {b}"
    );
}

#[test]
fn scalar_payout_range_must_be_finite_and_increasing() {
    for (low, high) in [("100", "100"), ("101", "100"), ("NaN", "100"), ("0", "inf")] {
        let params = format!(
            "perp:BTC|low:{low}|high:{high}|time:20300101-0000|seconds:90|priceDescription:the BTC perp"
        );
        assert!(resolve_template("template:scalarPrice", &params, &registry()).is_err());
    }
}
