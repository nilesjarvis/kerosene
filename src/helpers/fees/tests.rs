use super::{is_usd_stable_fee_token, non_perp_fee_usd};

#[test]
fn usd_stable_fee_tokens_are_recognized_case_insensitively() {
    for token in ["USDC", "usdc", "USDT0", "USDH", "USDE", "usde"] {
        assert!(is_usd_stable_fee_token(token), "{token} should be stable");
    }
    for token in ["HYPE", "UBTC", "PURR", ""] {
        assert!(!is_usd_stable_fee_token(token), "{token} is not stable");
    }
}

#[test]
fn stable_and_empty_fee_tokens_pass_through_unchanged() {
    assert_eq!(non_perp_fee_usd(0.25, "USDC", 40.0), 0.25);
    assert_eq!(non_perp_fee_usd(0.25, "USDT0", 40.0), 0.25);
    assert_eq!(non_perp_fee_usd(0.25, "", 40.0), 0.25);
}

#[test]
fn base_token_fees_convert_at_the_fill_price() {
    // A spot buy of HYPE at $40 charged 0.5 HYPE costs $20.
    assert_eq!(non_perp_fee_usd(0.5, "HYPE", 40.0), 20.0);
    // Rebates keep their sign through conversion.
    assert_eq!(non_perp_fee_usd(-0.5, "HYPE", 40.0), -20.0);
}
