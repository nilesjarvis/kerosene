use super::*;

#[test]
fn position_asset_move_is_side_adjusted() {
    assert_eq!(position_asset_move_pct(2.0, 100.0, 110.0), Some(10.0));
    assert_eq!(position_asset_move_pct(-2.0, 100.0, 90.0), Some(10.0));
    assert_eq!(position_asset_move_pct(-2.0, 100.0, 110.0), Some(-10.0));
}

#[test]
fn mark_can_be_reconstructed_from_wire_upnl() {
    assert_eq!(mark_from_wire_upnl(2.0, 100.0, Some(20.0)), Some(110.0));
    assert_eq!(mark_from_wire_upnl(-2.0, 100.0, Some(20.0)), Some(90.0));
    assert_eq!(mark_from_wire_upnl(0.0, 100.0, Some(20.0)), None);
}

#[test]
fn pct_from_basis_rejects_zero_basis() {
    assert_eq!(pct_from_basis(50.0, 1_000.0), Some(5.0));
    assert_eq!(pct_from_basis(50.0, 0.0), None);
}
