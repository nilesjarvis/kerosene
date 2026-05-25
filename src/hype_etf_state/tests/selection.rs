use super::*;

#[test]
fn selected_funds_respects_view() {
    let data = HypeEtfData {
        funds: vec![
            fund(HypeEtfTicker::Thyp, 1.0, 0.0),
            fund(HypeEtfTicker::Bhyp, 1.0, 0.0),
        ],
        warnings: Vec::new(),
    };

    let selected = data.selected_funds(HypeEtfView::Thyp);

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].ticker, HypeEtfTicker::Thyp);
}
