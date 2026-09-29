use super::*;

#[test]
fn pi_rpc_arguments_match_current_cli_contract() {
    assert_eq!(
        PI_RPC_ARGS,
        ["--mode", "rpc", "--no-session", "--thinking", "medium"]
    );
    let tools = PI_TOOL_ALLOWLIST.split(',').collect::<Vec<_>>();
    assert_eq!(tools.len(), 12);
    assert!(tools.contains(&"kerosene_set_chart_indicators"));
    assert!(tools.contains(&"kerosene_manage_chart_drawings"));
    assert!(tools.contains(&"kerosene_journal"));
    assert!(tools.contains(&"kerosene_pnl_card_match"));
    assert!(tools.iter().all(|tool| tool.starts_with("kerosene_")));
    assert!(
        !tools
            .iter()
            .any(|tool| matches!(*tool, "bash" | "read" | "write" | "edit"))
    );
}

#[test]
fn extension_requires_the_personalized_follow_up_contract() {
    assert!(EXTENSION_SOURCE.contains("<!-- KEROSENE_FOLLOW_UPS_V1"));
    assert!(EXTENSION_SOURCE.contains("exactly two concise, standalone questions"));
    assert!(EXTENSION_SOURCE.contains("Do not use generic prompts"));
}

#[test]
fn extension_contains_the_bounded_workspace_action_contract() {
    for requirement in [
        "name: \"kerosene_set_chart_indicators\"",
        "name: \"kerosene_manage_chart_drawings\"",
        "executionMode: \"sequential\"",
        "KEROSENE_HOST_ACTION_V1",
        "read kerosene_data with section workspace",
        "never treat a request for advice as permission to mutate",
        "Workspace mutation permission comes only from the current user message",
        "Use Unix epoch milliseconds and finite positive prices",
        "A locked drawing must be unlocked by the user before removal",
        "cannot trade, sign, place or cancel orders",
    ] {
        assert!(
            EXTENSION_SOURCE.contains(requirement),
            "missing embedded workspace action contract: {requirement}"
        );
    }
}

#[test]
fn packaged_pi_candidates_cover_supported_install_layouts() {
    let linux = packaged_pi_candidates(Path::new("/usr/bin"), "pi");
    let bundled_linux = PathBuf::from("/usr/lib/kerosene/pi/pi");
    let legacy_linux = PathBuf::from("/usr/bin/pi");
    assert!(linux.contains(&bundled_linux));
    assert!(
        linux.iter().position(|path| path == &bundled_linux)
            < linux.iter().position(|path| path == &legacy_linux)
    );

    let macos =
        packaged_pi_candidates(Path::new("/Applications/Kerosene.app/Contents/MacOS"), "pi");
    assert!(macos.contains(&PathBuf::from(
        "/Applications/Kerosene.app/Contents/Resources/pi/pi"
    )));

    let portable = packaged_pi_candidates(Path::new("/opt/kerosene"), "pi.exe");
    assert!(portable.contains(&PathBuf::from("/opt/kerosene/pi/pi.exe")));
}

#[test]
fn embedded_extension_contains_evidence_and_validation_contracts() {
    for requirement in [
        "Ground every material claim in evidence retrieved during the current turn.",
        "Never present an inference, hypothesis, or prior-turn value as a current fact.",
        "If sources conflict, expose the conflict instead of silently choosing one.",
        "Do not invent confidence percentages",
        "market_statistics",
        "excluded instead of treated as zero",
    ] {
        assert!(
            EXTENSION_SOURCE.contains(requirement),
            "missing embedded Assistant contract: {requirement}"
        );
    }
    assert!(
        !EXTENSION_SOURCE.contains("numericOrZero"),
        "financial tool code must not silently coerce missing values to zero"
    );
}
