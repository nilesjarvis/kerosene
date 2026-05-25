use super::*;

#[test]
fn optional_value_color_uses_invalid_color_only_for_missing_values() {
    let default_color = Color::from_rgb(0.1, 0.2, 0.3);
    let invalid_color = Color::from_rgb(0.8, 0.7, 0.6);

    assert_eq!(
        optional_value_color(Some(1.0), default_color, invalid_color),
        default_color
    );
    assert_eq!(
        optional_value_color::<f64>(None, default_color, invalid_color),
        invalid_color
    );
}

#[test]
fn signed_number_color_tracks_value_sign() {
    let theme = Theme::TokyoNight;

    assert_eq!(signed_number_color(1.0, &theme), theme.palette().success);
    assert_eq!(signed_number_color(-1.0, &theme), theme.palette().danger);
    assert_eq!(signed_number_color(0.0, &theme), theme.palette().text);
}
