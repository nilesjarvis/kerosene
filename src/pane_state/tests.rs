use super::*;

#[test]
fn panes_are_closeable() {
    assert!(PaneKind::Chart(0).can_be_closed());
    assert!(PaneKind::OrderEntry.can_be_closed());
}
