use super::*;

#[test]
fn history_signed_value_color_treats_zero_as_positive() {
    let theme = Theme::Dark;

    assert_eq!(
        history_signed_value_color(Some(1.0), &theme),
        theme.palette().success
    );
    assert_eq!(
        history_signed_value_color(Some(0.0), &theme),
        theme.palette().success
    );
    assert_eq!(
        history_signed_value_color(Some(-1.0), &theme),
        theme.palette().danger
    );
    assert_eq!(
        history_signed_value_color(None, &theme),
        theme.palette().warning
    );
}
