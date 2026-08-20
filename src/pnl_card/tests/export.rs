use super::*;

#[test]
fn filename_sanitizes_asset_ticker() {
    let filename = pnl_card_filename("xyz:BTC/USD");

    assert!(filename.starts_with("kerosene-xyz-btc-usd-pnl-card-"));
    assert!(filename.ends_with(".png"));
}

#[tokio::test]
async fn render_pnl_card_image_produces_expected_png_payload() {
    let state = position_state("BTC");
    let image =
        render_test_image(&state, sample_metrics(), Color::from_rgb8(0x50, 0xfa, 0x7b)).await;

    assert_eq!(image.width, 1200);
    assert_eq!(image.height, 675);
    assert_eq!(image.rgba.len(), 1200 * 675 * 4);
    assert!(image.png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(image.default_filename.starts_with("kerosene-btc-pnl-card-"));
    assert!(image.default_filename.ends_with(".png"));
}

#[tokio::test]
async fn positive_and_negative_exports_use_distinct_gradients() {
    let state = position_state("BTC");
    let positive =
        render_test_image(&state, sample_metrics(), Color::from_rgb8(0x50, 0xfa, 0x7b)).await;
    let negative =
        render_test_image(&state, sample_metrics(), Color::from_rgb8(0xff, 0x55, 0x55)).await;

    let interior = ((100 * positive.width + 100) * 4) as usize;
    assert_ne!(
        &positive.rgba[interior..interior + 64],
        &negative.rgba[interior..interior + 64]
    );
}
