use super::*;

#[test]
fn encode_png_rgba_rejects_wrong_buffer_size() {
    let err = error_or_panic(encode_png_rgba(2, 2, &[0; 4]), "wrong size");
    assert!(err.contains("unexpected size"));
}

#[test]
fn encode_png_rgba_produces_png_header() {
    let rgba = vec![255; 2 * 2 * 4];
    let png = png_or_panic(encode_png_rgba(2, 2, &rgba), "png");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
}
