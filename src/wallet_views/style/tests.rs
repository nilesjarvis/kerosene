use super::*;

#[test]
fn signed_value_color_tracks_sign_and_invalid_values() {
    let theme = Theme::Dark;

    assert_eq!(
        wallet_signed_value_color(Some(1.0), &theme),
        theme.palette().success
    );
    assert_eq!(
        wallet_signed_value_color(Some(-1.0), &theme),
        theme.palette().danger
    );
    assert_eq!(
        wallet_signed_value_color(Some(0.0), &theme),
        theme.extended_palette().background.weak.text
    );
    assert_eq!(
        wallet_signed_value_color(None, &theme),
        theme.palette().warning
    );
}
