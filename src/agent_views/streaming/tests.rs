use super::*;

#[test]
fn newest_stream_span_fades_without_changing_text() {
    let mut spans: Vec<iced::widget::text::Span<'static, markdown::Uri>> =
        vec![iced::widget::text::Span::new("Resolved newest ")];

    animate_latest_span(&mut spans, Color::WHITE, 0.0);

    assert_eq!(
        spans
            .iter()
            .map(|span| span.text.as_ref())
            .collect::<String>(),
        "Resolved newest "
    );
    assert!(
        spans
            .iter()
            .any(|span| span.color.is_some_and(|color| color.a < 0.5))
    );
}
