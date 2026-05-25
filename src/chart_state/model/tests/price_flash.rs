use super::*;

#[test]
fn last_price_flash_tracks_websocket_direction() {
    let mut instance = instance();

    instance.track_last_price_update(Some(100.0), 101.0, 42);
    assert_eq!(
        instance.last_price_flash,
        Some(PriceFlash {
            started_at_ms: 42,
            direction: PriceFlashDirection::Up,
            previous_close: 100.0,
        })
    );

    instance.track_last_price_update(Some(101.0), 99.0, 84);
    assert_eq!(
        instance.last_price_flash,
        Some(PriceFlash {
            started_at_ms: 84,
            direction: PriceFlashDirection::Down,
            previous_close: 101.0,
        })
    );
}

#[test]
fn last_price_flash_ignores_missing_or_unchanged_prices() {
    let mut instance = instance();

    instance.track_last_price_update(None, 101.0, 42);
    assert!(instance.last_price_flash.is_none());

    instance.track_last_price_update(Some(101.0), 101.0, 42);
    assert!(instance.last_price_flash.is_none());
}

#[test]
fn last_price_flash_expires_after_flash_window() {
    let mut instance = instance();

    instance.track_last_price_update(Some(100.0), 101.0, 1_000);
    assert!(instance.last_price_flash_is_active(1_000 + CHART_PRICE_FLASH_MS - 1));

    instance.clear_expired_last_price_flash(1_000 + CHART_PRICE_FLASH_MS);
    assert!(instance.last_price_flash.is_none());
}
