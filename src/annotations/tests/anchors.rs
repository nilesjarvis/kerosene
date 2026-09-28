use super::*;

#[test]
fn draggable_anchors_preserve_placement_order_and_unvalidated_values() {
    let start = (90, 12.5);
    let end = (10, -0.0);
    let third = (50, f64::NAN);
    let cases = [
        (AnnotationKind::HorizontalLevel { price: 12.5 }, vec![]),
        (AnnotationKind::VerticalLine { time: 90 }, vec![]),
        (AnnotationKind::TrendLine { start, end }, vec![start, end]),
        (AnnotationKind::Ray { start, end }, vec![start, end]),
        (
            AnnotationKind::ExtendedLine { start, end },
            vec![start, end],
        ),
        (AnnotationKind::Measure { start, end }, vec![start, end]),
        (
            AnnotationKind::Rectangle { a: end, b: start },
            vec![end, start],
        ),
        (
            AnnotationKind::Fib {
                kind: FibKind::Retracement,
                points: vec![start, end],
            },
            vec![start, end],
        ),
        (
            AnnotationKind::Fib {
                kind: FibKind::Extension,
                points: vec![start, end, third],
            },
            vec![start, end, third],
        ),
        (
            AnnotationKind::Fib {
                kind: FibKind::Retracement,
                points: vec![],
            },
            vec![],
        ),
        (
            AnnotationKind::Fib {
                kind: FibKind::Extension,
                points: vec![third],
            },
            vec![third],
        ),
        (
            AnnotationKind::Fib {
                kind: FibKind::Retracement,
                points: vec![third, end, start, end],
            },
            vec![third, end, start, end],
        ),
    ];
    for (kind, expected) in cases {
        let mut actual = Vec::new();
        for (time, price) in kind.anchor_points() {
            actual.push((time, price.to_bits()));
        }
        assert_eq!(
            actual,
            expected
                .into_iter()
                .map(|(time, price)| (time, price.to_bits()))
                .collect::<Vec<_>>()
        );
    }
}
