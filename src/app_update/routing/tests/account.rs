use super::*;

#[test]
fn account_and_order_routes_cover_overlapping_user_actions() {
    let window_id = window_id();

    assert_route(
        Message::ToggleHiddenPosition("BTC".to_string()),
        UpdateRoute::Account,
    );
    assert_route(
        Message::OpenPnlCard(PnlCardTarget::Position("BTC".to_string())),
        UpdateRoute::Account,
    );
    assert_route(
        Message::SetPnlCardDisplayMode(window_id, PnlCardDisplayMode::Both),
        UpdateRoute::Account,
    );
    assert_route(
        Message::SetPnlCardPercentMode(window_id, PnlCardPercentMode::Leveraged),
        UpdateRoute::Account,
    );
    assert_route(
        Message::TogglePnlCardPricePrivacy(window_id, true),
        UpdateRoute::Account,
    );
    assert_route(
        Message::TogglePnlCardPositionSize(window_id, true),
        UpdateRoute::Account,
    );
    assert_route(Message::CopyPnlCard(window_id), UpdateRoute::Account);
    assert_route(Message::PnlCardCopied(Ok(())), UpdateRoute::Account);
    assert_route(Message::SavePnlCard(window_id), UpdateRoute::Account);
    assert_route(Message::PnlCardSaved(Ok(None)), UpdateRoute::Account);
    assert_route(Message::AccountPickerRenameToggled(0), UpdateRoute::Account);
    assert_route(
        Message::AccountPickerLabelChanged(0, "Main".to_string()),
        UpdateRoute::Account,
    );
    assert_route(Message::RefreshAccountData, UpdateRoute::Account);
    assert_route(Message::DismissOrderStatus, UpdateRoute::Order);
    assert_route(
        Message::OrderBookPriceSelected {
            id: 7,
            price: "100.5".to_string(),
        },
        UpdateRoute::Order,
    );
}
