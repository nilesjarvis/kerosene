use super::*;

#[test]
fn ticker_label_text_sanitizes_and_truncates_to_available_width() {
    assert_eq!(
        ticker_label_text("ubtc/usdc:perp", "1H", 132, 1),
        "UBTC/USDC:PERP 1H"
    );
    assert_eq!(ticker_label_text("kPEPE@dex", "15m", 54, 1), "KPEPE 15M");
    assert_eq!(ticker_label_text("verylongticker", "1D", 48, 1), "VERYL 1D");
}

#[test]
fn draw_ticker_label_mutates_top_left_pixels() {
    let width = 160;
    let height = 80;
    let mut rgba = vec![0; width as usize * height as usize * 4];

    draw_ticker_label(&mut rgba, width, height, "BTC", "1H", test_label_style());

    assert!(rgba.iter().any(|value| *value != 0));
    let untouched_bottom_right = ((height as usize - 1) * width as usize + width as usize - 1) * 4;
    assert_eq!(
        &rgba[untouched_bottom_right..untouched_bottom_right + 4],
        &[0, 0, 0, 0]
    );
}

#[test]
fn draw_ticker_label_ignores_tiny_or_malformed_images() {
    let mut tiny = vec![0; 12];
    draw_ticker_label(&mut tiny, 2, 2, "BTC", "1H", test_label_style());
    assert_eq!(tiny, vec![0; 12]);

    let mut wrong_len = vec![0; 10];
    draw_ticker_label(&mut wrong_len, 160, 80, "BTC", "1H", test_label_style());
    assert_eq!(wrong_len, vec![0; 10]);
}
