use super::*;

#[test]
fn wallet_dex_label_marks_main_dex_when_missing() {
    assert_eq!(wallet_dex_label(""), "main");
    assert_eq!(wallet_dex_label("dex"), "dex");
}
