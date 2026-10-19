use super::*;

fn data(block: u64) -> PapertradeMarkets {
    PapertradeMarkets {
        indexed_through_block: block,
        markets: Vec::new(),
    }
}

#[test]
fn papertrade_catalog_coalesces_requests_caches_and_keeps_data_on_failure() {
    let now = Instant::now();
    let mut catalog = PapertradeCatalog::default();
    let id = catalog.begin_request(now, false).expect("first request");
    assert!(
        catalog
            .begin_request(now + Duration::from_secs(30), true)
            .is_none()
    );
    catalog.finish_request(id, Ok(data(1)), now);
    assert!(
        catalog
            .begin_request(now + Duration::from_secs(30), false)
            .is_none()
    );
    let retry = catalog
        .begin_request(now + Duration::from_secs(61), false)
        .expect("refresh");
    catalog.finish_request(id, Ok(data(9)), now);
    assert!(catalog.loading);
    catalog.finish_request(retry, Err("offline".into()), now + Duration::from_secs(61));
    assert_eq!(
        catalog.data.as_ref().expect("cache").indexed_through_block,
        1
    );
    assert_eq!(catalog.error.as_deref(), Some("offline"));
    assert!(
        catalog
            .begin_request(now + Duration::from_secs(62), false)
            .is_none()
    );
    assert!(
        catalog
            .begin_request(now + Duration::from_secs(62), true)
            .is_some()
    );
}

#[test]
fn papertrade_browser_defaults_are_independent_and_clear_rejects_old_requests() {
    let mut browser = MarketBrowserState::default();
    assert_eq!(
        browser.venue(MarketPicker::Chart(1)),
        MarketVenue::Hyperliquid
    );
    browser
        .venues
        .insert(MarketPicker::Chart(1), MarketVenue::Papertrade);
    assert_eq!(
        browser.venue(MarketPicker::ChartSecondary(1)),
        MarketVenue::Hyperliquid
    );
    assert_eq!(
        browser.venue(MarketPicker::Chart(2)),
        MarketVenue::Hyperliquid
    );
    let now = Instant::now();
    let old = browser
        .papertrade
        .begin_request(now, false)
        .expect("request");
    browser.clear();
    let current = browser
        .papertrade
        .begin_request(now, false)
        .expect("new request");
    browser.papertrade.finish_request(old, Ok(data(1)), now);
    assert!(browser.papertrade.loading);
    assert!(browser.papertrade.data.is_none());
    browser.papertrade.finish_request(current, Ok(data(2)), now);
    assert_eq!(
        browser.venue(MarketPicker::Chart(1)),
        MarketVenue::Hyperliquid
    );
    assert_eq!(
        browser
            .papertrade
            .data
            .expect("new data")
            .indexed_through_block,
        2
    );
}
